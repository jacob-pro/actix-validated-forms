use actix_multipart::{Field, Multipart, MultipartError};
use futures::{Future, Stream};
use futures::future::{err, Either};
use actix_web::web;
use actix_web::error::{BlockingError, PayloadError, ParseError};
use std::io::Write;
use tempfile::NamedTempFile;
use actix_web::http::header::{DispositionType};
use actix_web::web::BytesMut;
use super::{MultipartForm, MultipartFile, MultipartText, MultipartField};

// https://tools.ietf.org/html/rfc7578#section-1
// `content-type` defaults to text/plain
// However files must use appropriate MIME or application/octet-stream
// `filename` should be included but is not a must

#[derive(Clone)]
pub struct MultipartLoaderConfig {
    text_limit: usize,
    file_limit: u64,
}

impl MultipartLoaderConfig {

    pub fn text_limit(mut self, limit: usize) -> Self {
        self.text_limit = limit;
        self
    }

    pub fn file_limit(mut self, limit: u64) -> Self {
        self.file_limit = limit;
        self
    }

}

impl Default for MultipartLoaderConfig {
    fn default() -> Self {
        // Defaults are 1MB of text and 512MB of files
        MultipartLoaderConfig { text_limit: 1 * 1024 * 1024, file_limit: 512 * 1024 * 1024 }
    }
}

pub fn load(multipart: Multipart, config: MultipartLoaderConfig) -> impl Future<Item = MultipartForm, Error = MultipartError> {
    multipart.fold((Vec::new(), config.text_limit, config.file_limit),
                   move |(mut fields, text_budget, file_budget), field| {

        let cd = match field.content_disposition() {
            Some(cd) => cd,
            None => return Either::A(err(MultipartError::Parse(ParseError::Header))),
        };
        match cd.disposition {
            DispositionType::FormData => {},
            _ => return Either::A(err(MultipartError::Parse(ParseError::Header)))
        }
        let name= match cd.get_name() {
            Some(name) => name.to_owned(),
            None => return Either::A(err(MultipartError::Parse(ParseError::Header))),
        };

        let result = if field.content_type() == &mime::TEXT_PLAIN {
            Either::A(
                create_text(field, name, text_budget).map(move |(text, reduced_text_budget)| {
                    (MultipartField::Text(text), reduced_text_budget, file_budget)
                })
            )
        } else {
            let filename = cd.get_filename().map(|f| f.to_owned());
            Either::B(
                create_file(field, name, filename, file_budget).map(move |(file, reduced_file_budget)| {
                    (MultipartField::File(file), text_budget, reduced_file_budget)
                })
            )
        };

        Either::B(
            result.map(|(field, text_budget, file_budget)| {
                fields.insert(0, field);
                (fields, text_budget, file_budget)
            })
        )

    }).map(|k| { MultipartForm(k.0) })
}

// https://github.com/actix/examples/blob/master/multipart/src/main.rs
fn create_file(field: Field, name: String, filename: Option<String>, file_budget: u64) -> impl Future<Item = (MultipartFile, u64), Error = MultipartError> {
    let ntf = match NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => return Either::A(err(MultipartError::Payload(PayloadError::Io(e)))),
    };
    Either::B(
        field.fold((ntf, 0u64, file_budget), move |(file, written, budget), bytes| {
            let length = bytes.len() as u64;
            if budget < length {
                Either::A(err(MultipartError::Payload(PayloadError::Overflow)))
            } else {
                Either::B(
                    // fs operations are blocking, we have to execute writes on threadpool
                    web::block(move || {
                        file.as_file().write_all(bytes.as_ref()).map_err(|e| {
                            MultipartError::Payload(PayloadError::Io(e))
                        })?;
                        let written = written + length;
                        let remaining = budget - length;
                        Ok((file, written, remaining))
                    }).map_err(|e: BlockingError<MultipartError>| {
                        match e {
                            BlockingError::Error(e) => e,
                            BlockingError::Canceled => MultipartError::Incomplete,
                        }
                    })
                )
            }
        }).map(|(file, size, budget)| {
            (MultipartFile { file, name, filename, size }, budget)
        })
    )
}

fn create_text(field: Field, name: String, text_budget: usize) -> impl Future<Item = (MultipartText, usize), Error = MultipartError> {
    field
        .fold((BytesMut::new(), text_budget), move |(mut acc, budget), bytes| {
            let length = bytes.len();
            if budget < length {
                Err(MultipartError::Payload(PayloadError::Overflow))
            } else {
                acc.extend(bytes);
                Ok((acc, (budget - length)))
            }
        })
        .and_then(|(bytes, budget)| {
            //TODO: Currently only supports UTF-8
            //Consider looking at the charset header
            //And maybe also the _charset_ field
            String::from_utf8(bytes.to_vec())
                .map_err(|a| MultipartError::Parse(ParseError::Utf8(a.utf8_error())))
                .map(|text| (MultipartText { name, text }, budget) )
        })
}

