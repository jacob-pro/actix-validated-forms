use actix_web::ResponseError;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result};
use validator::ValidationErrors;

#[derive(Debug)]
pub enum ValidatedFormError<T: Debug + Display> {
    Deserialization(T),
    Validation(ValidationErrors),
}

impl<T: Debug + Display> Error for ValidatedFormError<T> {}
impl<T: Debug + Display> ResponseError for ValidatedFormError<T> {}

impl<T: Debug + Display> Display for ValidatedFormError<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ValidatedFormError::Validation(e) => Display::fmt(&e, f),
            ValidatedFormError::Deserialization(e) => Display::fmt(&e, f),
        }
    }
}
