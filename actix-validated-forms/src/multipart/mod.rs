mod extractor;
mod load;
#[cfg(test)]
mod test;

pub use extractor::*;
pub use load::*;

use actix_web::http::StatusCode;
use actix_web::ResponseError;
use err_derive::Error;
use std::str::FromStr;
use tempfile::NamedTempFile;

pub type Multiparts = Vec<MultipartField>;

#[derive(Debug)]
pub struct MultipartFile {
    pub file: NamedTempFile,
    pub name: String,
    pub filename: Option<String>,
    pub size: u64,
    pub reported_mime: mime::Mime,
}

#[derive(Debug)]
pub struct MultipartText {
    pub name: String,
    pub text: String,
}

#[derive(Debug)]
pub enum MultipartField {
    File(MultipartFile),
    Text(MultipartText),
}

#[derive(Debug, Error)]
pub enum GetError {
    #[error(display = "Field \"{}\" not found", _0)]
    NotFound(String),
    #[error(display = "Field \"{}\" couldn't be converted into {}", _0, _1)]
    TypeError(String, String),
    #[error(display = "Duplicate values found for field \"{}\"", _0)]
    DuplicateField(String),
}

impl ResponseError for GetError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

pub trait MultipartType
where
    Self: std::marker::Sized,
{
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError>;
}

pub trait MultipartTypeFromString: FromStr {}

macro_rules! impl_t {
    (for $($t:ty),+) => {
        $(impl MultipartTypeFromString for $t { })*
    }
}
impl_t!(for i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, String);

impl<T: MultipartTypeFromString> MultipartType for T {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<T>::get(form, field_name)?;
        match matches.len() {
            0 => Err(GetError::NotFound(field_name.into())),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl<T: MultipartTypeFromString> MultipartType for Option<T> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<T>::get(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl<T: MultipartTypeFromString> MultipartType for Vec<T> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::new();
        for i in form {
            match i {
                MultipartField::File(_) => {}
                MultipartField::Text(x) => {
                    if x.name == field_name {
                        let y: T = x.text.parse().map_err(|_| GetError::TypeError(field_name.into(), std::any::type_name::<T>().into()))?;
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

impl MultipartType for Option<MultipartFile> {
    fn get(form: &mut Multiparts, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<MultipartFile>::get(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(GetError::DuplicateField(field_name.into())),
        }
    }
}

impl MultipartType for Vec<MultipartFile> {
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
