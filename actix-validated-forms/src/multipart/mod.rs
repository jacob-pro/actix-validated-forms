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
    MultipleItems,
}

pub trait MultipartType
where
    Self: std::marker::Sized,
{
    fn get(form: &MultipartForm, field_name: &str) -> Result<Self, GetError>;
}

impl<T: FromStr> MultipartType for T {
    fn get(form: &MultipartForm, field_name: &str) -> Result<Self, GetError> {
        let mut matches = Vec::new();
        for i in form {
            match i {
                MultipartField::File(_) => {},
                MultipartField::Text(x) => {
                    if x.name == field_name {
                        let y: T = x.text.parse().map_err(|e| GetError::TypeError)?;
                        matches.push(y);
                    }
                },
            }
        }
        return match matches.len() {
            0 => {Err(GetError::NotFound)}
            1 => {Ok(matches.pop().unwrap())}
            _ => {Err(GetError::MultipleItems)}
        }
    }
}
