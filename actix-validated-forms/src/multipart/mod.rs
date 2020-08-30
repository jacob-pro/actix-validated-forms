mod load;
mod extractor;

pub use load::*;
pub use extractor::*;

use tempfile::NamedTempFile;

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

    pub fn get<T>(&self, named: &str) -> Result<T, GetError> {
        println!("getting {}", named);
        Err(GetError::NotFound)
    }

}
