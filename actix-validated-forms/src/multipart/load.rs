use super::{MultipartField, MultipartFile, MultipartText, Multiparts};
use actix_multipart::MultipartError;
use actix_web::error::{BlockingError, ParseError, PayloadError};
use actix_web::http::header;
use actix_web::http::header::DispositionType;
use actix_web::web::{self, BytesMut};
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use std::io::Write;
use tempfile::NamedTempFile;

// https://tools.ietf.org/html/rfc7578#section-1
// `content-type` defaults to text/plain
// files SHOULD use appropriate mime or application/octet-stream
// `filename` SHOULD be included but is not a MUST

/// Configuration options when loading a multipart form
#[derive(Clone)]
pub struct MultipartLoadConfig {
    text_limit: usize,
    file_limit: u64,
    max_parts: usize,
}

impl MultipartLoadConfig {
    /// Maximum total bytes of text (will be loaded into memory) - default 1 MiB
    pub fn text_limit(mut self, limit: usize) -> Self {
        self.text_limit = limit;
        self
    }

    /// Maximum total bytes of file upload (will be written to temporary file) - default 512 MiB
    pub fn file_limit(mut self, limit: u64) -> Self {
        self.file_limit = limit;
        self
    }

    /// Maximum parts the form may contain - default 1000
    pub fn max_parts(mut self, max: usize) -> Self {
        self.max_parts = max;
        self
    }
}

impl Default for MultipartLoadConfig {
    fn default() -> Self {
        // Defaults are 1MB of text and 512MB of files
        MultipartLoadConfig {
            text_limit: 1 * 1024 * 1024,
            file_limit: 512 * 1024 * 1024,
            max_parts: 1000,
        }
    }
}

/// Use to load a multipart form from an Actix Multipart request
///
/// This is an asynchronous operation, blocking IO such as writing an uploaded file
/// to disk will be done on a background thread pool (using `actix_web::web::block`)
///
/// # Example
/// ```
/// # use actix_validated_forms::multipart::{load_parts, MultipartLoadConfig};
/// # use actix_web::{HttpResponse, Error};
/// async fn route(payload: actix_multipart::Multipart) -> Result<HttpResponse, Error> {
///     let mut form = load_parts(payload, MultipartLoadConfig::default()).await?;
///     # unimplemented!() }
/// ```
pub async fn load_parts(
    mut payload: actix_multipart::Multipart,
    config: MultipartLoadConfig,
) -> Result<Multiparts, MultipartError> {
    let mut parts = Multiparts::new();
    let mut text_budget = config.text_limit;
    let mut file_budget = config.file_limit;

    while let Ok(Some(field)) = payload.try_next().await {
        if parts.len() >= config.max_parts {
            return Err(MultipartError::Payload(PayloadError::Overflow));
        }
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

        // We need to default to TEXT_PLAIN however actix content_type() defaults to APPLICATION_OCTET_STREAM
        let content_type = if field.headers().get(&header::CONTENT_TYPE).is_none() {
            mime::TEXT_PLAIN
        } else {
            field.content_type().clone()
        };

        let item = if content_type == mime::TEXT_PLAIN && cd.get_filename().is_none() {
            let (r, size) = create_text(field, name, text_budget).await?;
            text_budget = text_budget - size;
            MultipartField::Text(r)
        } else {
            let filename = cd.get_filename().map(|f| f.to_owned());
            let r = create_file(field, name, filename, file_budget, content_type).await?;
            file_budget = file_budget - r.size;
            MultipartField::File(r)
        };
        parts.push(item);
    }
    Ok(parts)
}

async fn create_file(
    mut field: actix_multipart::Field,
    name: String,
    filename: Option<String>,
    max_size: u64,
    mime: mime::Mime,
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
        size: written,
        name,
        filename,
        mime,
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
    Ok((MultipartText { name, text }, written))
}
