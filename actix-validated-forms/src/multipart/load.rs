use super::{MultipartField, MultipartFile, MultipartForm, MultipartText};
use actix_multipart::MultipartError;
use actix_web::error::{BlockingError, ParseError, PayloadError};
use actix_web::http::header::DispositionType;
use actix_web::web::{self, BytesMut};
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use std::io::Write;
use tempfile::NamedTempFile;

// https://tools.ietf.org/html/rfc7578#section-1
// `content-type` defaults to text/plain
// However files must use appropriate MIME or application/octet-stream
// `filename` should be included but is not a must

#[derive(Clone)]
pub struct MultipartLoadConfig {
    text_limit: usize,
    file_limit: u64,
}

impl MultipartLoadConfig {
    pub fn text_limit(mut self, limit: usize) -> Self {
        self.text_limit = limit;
        self
    }

    pub fn file_limit(mut self, limit: u64) -> Self {
        self.file_limit = limit;
        self
    }
}

impl Default for MultipartLoadConfig {
    fn default() -> Self {
        // Defaults are 1MB of text and 512MB of files
        MultipartLoadConfig {
            text_limit: 1 * 1024 * 1024,
            file_limit: 512 * 1024 * 1024,
        }
    }
}

// https://github.com/actix/examples/blob/master/multipart/src/main.rs
pub async fn load(
    mut payload: actix_multipart::Multipart,
    config: MultipartLoadConfig,
) -> Result<MultipartForm, MultipartError> {
    let mut form = MultipartForm::new();
    let mut text_budget = config.text_limit;
    let mut file_budget = config.file_limit;
    while let Ok(Some(field)) = payload.try_next().await {
        let cd = match field.content_disposition() {
            Some(cd) => cd,
            None => return Err(MultipartError::Parse(ParseError::Header)),
        };
        match cd.disposition {
            DispositionType::FormData => {}
            _ => return Err(MultipartError::Parse(ParseError::Header)),
        }
        let name = match cd.get_name() {
            Some(name) => name.to_owned(),
            None => return Err(MultipartError::Parse(ParseError::Header)),
        };
        let content_type = field.content_type().clone();
        let item = if content_type == mime::TEXT_PLAIN {
            let (r, size) = create_text(field, name, text_budget).await?;
            text_budget = text_budget - size;
            MultipartField::Text(r)
        } else {
            let filename = cd.get_filename().map(|f| f.to_owned());
            let r = create_file(field, name, filename, file_budget, content_type).await?;
            file_budget = file_budget - r.size;
            MultipartField::File(r)
        };
        form.push(item);
    }
    Ok(form)
}

async fn create_file(
    mut field: actix_multipart::Field,
    name: String,
    filename: Option<String>,
    max_size: u64,
    reported_mime: mime::Mime,
) -> Result<MultipartFile, MultipartError> {
    let mut written = 0;
    let mut budget = max_size;
    let mut ntf = match NamedTempFile::new() {
        Ok(file) => file,
        Err(e) => return Err(MultipartError::Payload(PayloadError::Io(e))),
    };

    while let Some(chunk) = field.next().await {
        let bytes = chunk?;
        let length = bytes.len() as u64;
        if budget < length {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }
        ntf = web::block(move || {
            ntf.as_file()
                .write_all(bytes.as_ref())
                .map(|_| ntf)
                .map_err(|e| MultipartError::Payload(PayloadError::Io(e)))
        })
        .map_err(|e: BlockingError<MultipartError>| match e {
            BlockingError::Error(e) => e,
            BlockingError::Canceled => MultipartError::Incomplete,
        })
        .await?;

        written = written + length;
        budget = budget - length;
    }
    Ok(MultipartFile {
        file: ntf,
        name,
        filename,
        size: written,
        reported_mime,
    })
}

async fn create_text(
    mut field: actix_multipart::Field,
    name: String,
    max_length: usize,
) -> Result<(MultipartText, usize), MultipartError> {
    let mut written = 0;
    let mut budget = max_length;
    let mut acc = BytesMut::new();

    while let Some(chunk) = field.next().await {
        let bytes = chunk?;
        let length = bytes.len();
        if budget < length {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }
        acc.extend(bytes);
        written = written + length;
        budget = budget - length;
    }
    //TODO: Currently only supports UTF-8, consider looking at the charset header and _charset_ field
    let text = String::from_utf8(acc.to_vec())
        .map_err(|a| MultipartError::Parse(ParseError::Utf8(a.utf8_error())))?;
    Ok((MultipartText { name, text }, budget))
}
