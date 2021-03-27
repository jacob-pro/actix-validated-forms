mod extractor;
mod load;
#[cfg(test)]
mod test;

pub use extractor::*;
pub use load::*;

use actix_web::http::StatusCode;
use actix_web::ResponseError;
use err_derive::Error;
use std::ffi::OsStr;
use std::path::Path;
use std::str::FromStr;
use tempfile::NamedTempFile;

/// A Multipart form is just an array of Multipart Fields
///
/// Use with the `MultipartType` (and `MultipartTypeSpecial`) traits for easily accessing a given
/// field/part by name
/// # Example
/// ```
/// let parts: Multiparts = load_parts(payload, MultipartLoadConfig::default()).await?;
/// let int_val: i64 = MultipartType::get(&mut parts, "field_name")?;
/// let str_val: String = MultipartType::get(&mut parts, "field_name")?;
/// ```
pub type Multiparts = Vec<MultipartField>;

/// Structure used to represent a File upload in a mulipart form
///
/// A body part is treated as a file upload if the Content-Type header is set to anything
/// other than `text/plain` or a `filename` is specified in the content disposition header.
#[derive(Debug)]
pub struct MultipartFile {
    /// The file data itself stored as a temporary file on disk
    pub file: NamedTempFile,
    /// The size in bytes of the file
    pub size: u64,
    /// The name of the field in the multipart form
    pub name: String,
    /// The `filename` value in the `Content-Disposition` header
    pub filename: Option<String>,
    /// The Content-Type specified as reported in the uploaded form
    /// DO NOT trust this as being accurate
    pub mime: mime::Mime,
}

impl MultipartFile {
    /// Get the extension portion of the `filename` value in the `Content-Disposition` header
    pub fn get_extension(&self) -> Option<&str> {
        self.filename
            .as_ref()
            .and_then(|f| Path::new(f.as_str()).extension().and_then(OsStr::to_str))
    }
}

/// Structure used to represent a Text field in a mulipart form
///
/// A body part is treated as text if the Content-Type header is equal to `text/plain`
/// (or otherwise unspecified - since `text/plain` is the default), and no `filename` is
/// specified in the content disposition header.
#[derive(Debug)]
pub struct MultipartText {
    /// The name of the field in the multipart form
    pub name: String,
    /// The text body of the field / part
    pub text: String,
}

#[derive(Debug)]
pub enum MultipartField {
    File(MultipartFile),
    Text(MultipartText),
}

#[derive(Debug, Error)]
pub enum GetError {
    /// If this field is optional try using Option<T>::get() instead
    #[error(display = "Field '{}' not found", _0)]
    NotFound(String),
    #[error(display = "Field '{}' couldn't be converted into {}", _0, _1)]
    TypeError(String, String),
    /// If this field is actually an array of uploaded items try using Vec<T>::get() instead
    #[error(display = "Duplicate values found for field '{}'", _0)]
    DuplicateField(String),
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

/// Allows retrieving a specific named field/part from a Multipart form
pub trait MultipartType
where
    Self: std::marker::Sized,
{
    /// Attempt to retrieve a named field/part from the Multipart form
    ///
    /// Implementations are provided for any type that implements `FromStr`
    /// # Example
    /// ```
    /// let int_val: i64 = MultipartType::get(&mut form, "field_name")?;
    /// let str_val: String = MultipartType::get(&mut form, "field_name")?;
    /// ```
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError>;
}

/// A work-around while Rust trait [specialization] is not yet available
///
/// [specialization]: https://rust-lang.github.io/rfcs/1210-impl-specialization.html
pub trait MultipartTypeSpecial
where
    Self: std::marker::Sized,
{
    /// Attempt to retrieve a named field/part from the Multipart form
    ///
    /// Where the type is either a `Vec<T>` or `Option<T>` where `T` implements `FromStr`
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError>;
}

impl<T: FromStr> MultipartType for T {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<T>::get(form, field_name)?;
        match matches.len() {
            0 => Err(GetError::NotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl<T: FromStr> MultipartTypeSpecial for Option<T> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<T>::get(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl<T: FromStr> MultipartTypeSpecial for Vec<T> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::new();
        for i in form {
            match i {
                MultipartField::File(_) => {}
                MultipartField::Text(x) => {
                    if x.name == field_name {
                        let y: T = x.text.parse().map_err(|_| {
                            GetError::TypeError(
                                field_name.into(),
                                std::any::type_name::<T>().into(),
                            )
                        })?;
                        matches.push(y);
                    }
                }
            }
        }
        Ok(matches)
    }
}

impl MultipartType for MultipartFile {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<MultipartFile>::get(form, field_name)?;
        match matches.len() {
            0 => Err(GetError::NotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl MultipartTypeSpecial for Option<MultipartFile> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<MultipartFile>::get(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl MultipartTypeSpecial for Vec<MultipartFile> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut indexes = Vec::new();
        for (idx, item) in form.iter().enumerate() {
            match item {
                MultipartField::Text(_) => {}
                MultipartField::File(x) => {
                    if x.name == field_name {
                        indexes.push(idx)
                    }
                }
            }
        }
        Ok(indexes
            .iter()
            .rev()
            .map(|idx| match form.remove(*idx) {
                MultipartField::File(x) => x,
                MultipartField::Text(_) => panic!(),
            })
            .collect())
    }
}
