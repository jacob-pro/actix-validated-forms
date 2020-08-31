use super::{load_parts, MultipartLoadConfig, Multiparts};
use crate::error::ValidatedFormError;
use crate::multipart::GetError;
use actix_multipart::{Multipart, MultipartError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::ops;
use std::rc::Rc;
use validator::Validate;

pub struct ValidatedMultipartForm<T>(pub T);

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
            .map(move |e| match e {
                Ok(item) => Ok(ValidatedMultipartForm(item)),
                Err(e) => {
                    if let Some(err) = config.error_handler {
                        Err((*err)(e, &req2))
                    } else {
                        Err(Self::Error::from(e))
                    }
                }
            })
            .boxed_local()
    }
}

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
