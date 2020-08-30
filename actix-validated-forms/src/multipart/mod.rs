mod load;
mod extractor;

pub use load::*;
pub use extractor::*;

use tempfile::NamedTempFile;
use std::str::FromStr;

#[derive(Debug)]
pub struct MultipartForm(Vec<MultipartField>);

#[derive(Debug)]
pub struct MultipartFile {
    file: NamedTempFile,
    name: String,
    filename: Option<String>,
    size: u64,
}

#[derive(Debug)]
pub struct MultipartText {
    name: String,
    text: String,
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

impl MultipartForm {
    pub fn new(x: Vec<MultipartField>) -> Self {
        MultipartForm(x)
    }
}

pub trait MultipartType
    where Self: std::marker::Sized
{
    fn get(form: &MultipartForm, field_name: &str) -> Result<Self, GetError>;
}

impl<T: FromStr> MultipartType for T {
    fn get(form: &MultipartForm, field_name: &str) -> Result<Self, GetError> {
        Err(GetError::NotFound)
    }
}
