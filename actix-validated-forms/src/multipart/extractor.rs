use super::{load_parts, MultipartLoadConfig, Multiparts};
use crate::error::ValidatedFormError;
use crate::multipart::GetError;
use actix_multipart::{Multipart, MultipartError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;
use futures::{FutureExt, TryFutureExt};
use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::ops;
use std::rc::Rc;
use validator::Validate;

/// Validated extractor for a HTTP Multipart request
///
/// # Example
/// First define a structure to represent the form that implements `FromMultipart` and
/// `validator::Validate` traits. Then use the extractor in your route
///
/// ```
/// # #[macro_use] extern crate validator_derive;
/// # fn main() {
/// # use actix_validated_forms_derive::FromMultipart;
/// # use validator::Validate;
/// #[derive(FromMultipart, Validate)]
/// struct MultipartUpload {
///    #[validate(length(max = 4096))]
///    description: String,
///    image: MultipartFile,
/// }
/// # use actix_web::{HttpResponse};
/// # use actix_validated_forms::multipart::{MultipartFile, ValidatedMultipartForm};
///
/// async fn route(
///     form: ValidatedMultipartForm<MultipartUpload>,
/// ) -> HttpResponse {
///     let img_bytes = std::fs::read(form.image.file.path()).unwrap();
///     # unimplemented!(); }
/// # }
/// ```
pub struct ValidatedMultipartForm<T: Validate>(pub T);

impl<T: Validate> ValidatedMultipartForm<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Validate> ops::Deref for ValidatedMultipartForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Validate> ops::DerefMut for ValidatedMultipartForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> FromRequest for ValidatedMultipartForm<T>
where
    T: TryFrom<Multiparts, Error = GetError> + Validate + 'static,
{
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = ValidatedMultipartFormConfig;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let config = req
            .app_data::<Self::Config>()
            .map(|c| c.clone())
            .unwrap_or(Self::Config::default());

        // Create actix_multipart::Multipart from HTTP Request
        let x = Multipart::new(req.headers(), payload.take());
        // Read into a Multiparts (a vector of fields and temp files on disk)
        load_parts(x, config.config.clone())
            .map(move |res| match res {
                Ok(item) => {
                    // Try to parse the multiparts into the struct T
                    let x = T::try_from(item).map_err(|e| {
                        ValidatedFormError::Deserialization(MultipartErrorWrapper::Deserialization(
                            e,
                        ))
                    })?;
                    // And then validate the struct T
                    x.validate()
                        .map_err(|e| ValidatedFormError::Validation(e))?;
                    Ok(x)
                }
                Err(e) => Err(ValidatedFormError::Deserialization(
                    MultipartErrorWrapper::Multipart(e),
                )),
            })
            .map_ok(ValidatedMultipartForm)
            .map_err(move |e| {
                if let Some(err) = config.error_handler {
                    (*err)(e, &req2)
                } else {
                    Self::Error::from(e)
                }
            })
            .boxed_local()
    }
}

/// Configure the behaviour of the ValidatedMultipartForm extractor
///
/// # Usage
/// Add a `ValidatedFormConfig` to your actix app data
/// ```
/// # use actix_web::web::scope;
/// use actix_validated_forms::multipart::{ValidatedMultipartFormConfig, MultipartLoadConfig};
/// scope("/").app_data(
///     ValidatedMultipartFormConfig::default().config(
///         MultipartLoadConfig::default().file_limit(25 * 1024 * 1024) // 25 MiB
///     )
/// );
/// ```
#[derive(Clone)]
pub struct ValidatedMultipartFormConfig {
    config: MultipartLoadConfig,
    error_handler: Option<
        Rc<dyn Fn(ValidatedFormError<MultipartErrorWrapper>, &HttpRequest) -> actix_web::Error>,
    >,
}

impl ValidatedMultipartFormConfig {
    pub fn config(mut self, config: MultipartLoadConfig) -> Self {
        self.config = config;
        self
    }
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(ValidatedFormError<MultipartErrorWrapper>, &HttpRequest) -> actix_web::Error
            + 'static,
    {
        self.error_handler = Some(Rc::new(f));
        self
    }
}

impl Default for ValidatedMultipartFormConfig {
    fn default() -> Self {
        ValidatedMultipartFormConfig {
            config: Default::default(),
            error_handler: None,
        }
    }
}

#[derive(Debug)]
pub enum MultipartErrorWrapper {
    Multipart(MultipartError),
    Deserialization(GetError),
}

impl std::error::Error for MultipartErrorWrapper {}

impl Display for MultipartErrorWrapper {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            MultipartErrorWrapper::Multipart(e) => Display::fmt(&e, f),
            MultipartErrorWrapper::Deserialization(e) => Display::fmt(&e, f),
        }
    }
}
