use crate::error::ValidatedFormError;
use actix_web::dev::Payload;
use actix_web::error::QueryPayloadError;
use actix_web::{FromRequest, HttpRequest};
use serde::de::DeserializeOwned;
use std::rc::Rc;
use std::{fmt, ops};
use validator::Validate;

pub struct ValidatedQuery<T: Validate>(pub T);

impl<T: Validate> ValidatedQuery<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Validate> ops::Deref for ValidatedQuery<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Validate> ops::DerefMut for ValidatedQuery<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> FromRequest for ValidatedQuery<T>
where
    T: Validate + DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = Result<Self, Self::Error>;
    type Config = ValidatedQueryConfig;

    #[inline]
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let config = req
            .app_data::<ValidatedQueryConfig>()
            .map(|c| c.clone())
            .unwrap_or(ValidatedQueryConfig::default());

        serde_urlencoded::from_str::<T>(req.query_string())
            .map_err(move |e| {
                ValidatedFormError::Deserialization(QueryPayloadError::Deserialize(e))
            })
            .and_then(|c: T| {
                c.validate()
                    .map(|_| c)
                    .map_err(|e| ValidatedFormError::Validation(e))
            })
            .map_err(move |e| {
                if let Some(err) = config.error_handler {
                    (*err)(e, &req)
                } else {
                    e.into()
                }
            })
            .map(ValidatedQuery)
    }
}

impl<T: Validate + fmt::Debug> fmt::Debug for ValidatedQuery<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Validate + fmt::Display> fmt::Display for ValidatedQuery<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone)]
pub struct ValidatedQueryConfig {
    error_handler:
        Option<Rc<dyn Fn(ValidatedFormError<QueryPayloadError>, &HttpRequest) -> actix_web::Error>>,
}

impl ValidatedQueryConfig {
    /// Set custom error handler
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(ValidatedFormError<QueryPayloadError>, &HttpRequest) -> actix_web::Error + 'static,
    {
        self.error_handler = Some(Rc::new(f));
        self
    }
}

impl Default for ValidatedQueryConfig {
    fn default() -> Self {
        ValidatedQueryConfig {
            error_handler: None,
        }
    }
}
