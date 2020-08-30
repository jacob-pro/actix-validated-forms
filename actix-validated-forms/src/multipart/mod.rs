//mod extractor;
mod load;

//pub use extractor::*;
pub use load::*;

use std::str::FromStr;
use tempfile::NamedTempFile;

pub type MultipartForm = Vec<MultipartField>;

#[derive(Debug)]
pub struct MultipartFile {
    pub file: NamedTempFile,
    pub name: String,
    pub filename: Option<String>,
    pub size: u64,
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

#[derive(Debug)]
pub enum GetError {
    NotFound,
    TypeError,
    DuplicateField,
}

pub trait MultipartType
where
    Self: std::marker::Sized,
{
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError>;
}

pub trait TypeFromString: FromStr {}

macro_rules! impl_t {
    (for $($t:ty),+) => {
        $(impl TypeFromString for $t {
        })*
    }
}
impl_t!(for i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, String);

impl<T: TypeFromString> MultipartType for T {
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<T>::get(form, field_name)?;
        match matches.len() {
            0 => Err(GetError::NotFound),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(GetError::DuplicateField),
        }
    }
}

impl<T: TypeFromString> MultipartType for Option<T> {
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<T>::get(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(GetError::DuplicateField),
        }
    }
}

impl<T: TypeFromString> MultipartType for Vec<T> {
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::new();
        for i in form {
            match i {
                MultipartField::File(_) => {}
                MultipartField::Text(x) => {
                    if x.name == field_name {
                        let y: T = x.text.parse().map_err(|_| GetError::TypeError)?;
                        matches.push(y);
                    }
                }
            }
        }
        Ok(matches)
    }
}

impl MultipartType for MultipartFile {
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<MultipartFile>::get(form, field_name)?;
        match matches.len() {
            0 => Err(GetError::NotFound),
            1 => Ok(matches.pop().unwrap()),
            _ => Err(GetError::DuplicateField),
        }
    }
}

impl MultipartType for Option<MultipartFile> {
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::<MultipartFile>::get(form, field_name)?;
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches.pop().unwrap())),
            _ => Err(GetError::DuplicateField),
        }
    }
}

impl MultipartType for Vec<MultipartFile> {
    fn get(form: &mut MultipartForm, field_name: &str) -> Result<Self, GetError> {
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
