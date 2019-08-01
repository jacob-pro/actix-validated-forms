use actix_multipart::{Field, Multipart, MultipartError};
use futures::{Future, Stream};
use futures::future::{err, Either};
use actix_web::web;
use actix_web::error::{BlockingError, PayloadError, ParseError};
use std::io::Write;
use tempfile::NamedTempFile;
use actix_web::http::header::{DispositionType};


pub struct MultipartFile {
    file: NamedTempFile,
    name: String,
    filename: String,
}

pub struct MultipartText {
    name: String,
    text: String,
}

pub enum MultipartField {
    File(MultipartFile),
    Text(MultipartText),
}


pub fn load(multipart: Multipart, field_limit: usize, file_limit: u64) -> impl Future<Item = Vec<MultipartField>, Error = MultipartError> {
    multipart
        .map(|field| {
            handle_field(field).into_stream()
        })
        .flatten()
        .collect()
}

fn handle_field(field: Field) -> Box<dyn Future<Item = MultipartField, Error = MultipartError>> {
    let cd = match field.content_disposition() {
        Some(cd) => cd,
        None => return Box::new(err(MultipartError::Parse(ParseError::Header))),
    };
    match cd.disposition {
        DispositionType::FormData => {},
        _ => return Box::new(err(MultipartError::Parse(ParseError::Header)))
    }
    let name= match cd.get_name() {
        Some(name) => name.to_owned(),
        None => return Box::new(err(MultipartError::Parse(ParseError::Header))),
    };
    match cd.get_filename() {
        None => Box::new(create_file(field, name, "".to_owned()).map(|f| MultipartField::File(f))),
        Some(filename) => Box::new(create_file(field, name, filename.to_owned()).map(|f| MultipartField::File(f))),
    }
}

// https://github.com/actix/examples/blob/master/multipart/src/main.rs
fn create_file(field: Field, name: String, filename: String) -> impl Future<Item = MultipartFile, Error = MultipartError> {
    let ntf = match NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => return Either::A(err(MultipartError::Payload(PayloadError::Io(e)))),
    };
    Either::B(
        field
            .fold((ntf, 0i64), move |(file, written), bytes| {
                // fs operations are blocking, we have to execute writes on threadpool
                web::block(move || {
                    file.as_file().write_all(bytes.as_ref()).map_err(|e| {
                        MultipartError::Payload(PayloadError::Io(e))
                    })?;
                    let written = written + bytes.len() as i64;
                    Ok((file, written))
                }).map_err(|e: BlockingError<MultipartError>| {
                    match e {
                        BlockingError::Error(e) => e,
                        BlockingError::Canceled => MultipartError::Incomplete,
                    }
                })
            })
            .map(|(file, _)| {
                MultipartFile { file, name, filename }
            })
    )
}
