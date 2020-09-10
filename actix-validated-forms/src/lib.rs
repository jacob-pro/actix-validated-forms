#[cfg(test)]
#[macro_use]
extern crate validator_derive;

pub mod error;
pub mod form;
pub mod multipart;
pub mod query;

pub use tempfile;
pub use validator;

// Re-export derive
#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate actix_validated_forms_derive;
#[cfg(feature = "derive")]
#[doc(hidden)]
pub use actix_validated_forms_derive::*;
