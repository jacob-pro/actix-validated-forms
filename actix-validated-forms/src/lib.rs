//! Validated HTTP forms and query extractors for the [Actix-Web] framework
//! using the [validator] crate for struct validation.
//!
//! Also adds an easy to use HTTP multipart form extractor (with validation)
//! that generates temporary files on disk using the [tempfile] crate with similar
//! behaviour to the php [$_FILES] variable in php
//!
//! [Actix-Web]: https://github.com/actix/actix-web
//! [validator]: https://github.com/Keats/validator
//! [tempfile]: https://github.com/Stebalien/tempfile
//! [$_FILES]: https://www.php.net/manual/en/reserved.variables.files.php#89674

#[cfg(test)]
#[macro_use]
extern crate validator_derive;

pub mod error;
/// Validated extractor for an application/x-www-form-urlencoded HTTP request body
pub mod form;
/// Validated extractor for a multipart/form-data HTTP request body
pub mod multipart;
/// Validated extractor for a Url Encoded HTTP Query String
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
pub use actix_validated_forms_derive::FromMultipart;
