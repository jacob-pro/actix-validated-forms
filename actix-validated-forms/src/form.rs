use crate::error::ValidatedFormError;
use actix_web::dev::{Payload, UrlEncoded};
use actix_web::error::UrlencodedError;
use actix_web::{FromRequest, HttpRequest};
use futures::future::{self, FutureExt, LocalBoxFuture};
use futures::TryFutureExt;
use serde::de::DeserializeOwned;
use std::rc::Rc;
use std::{fmt, ops};
use validator::Validate;

/// Validated extractor for an application/x-www-form-urlencoded HTTP request body
///
/// # Example
/// First define a structure to represent the form that implements `serde::Deserialize` and
/// `validator::Validate` traits.
/// ```
/// #[derive(Deserialize, Validate)]
/// struct ExampleForm {
///     #[validate(length(min = 1, max = 5))]
///     field: String,
/// }
/// ```
/// Use the extractor in your route:
/// ```
/// async fn route(
///     form: ValidatedForm<ExampleForm>,
/// ) -> impl Responder { ... }
/// ```
/// Just like the `actix_web::web::Form` when the body of route is executed `form` can be
/// dereferenced to an `ExampleForm`, however it has the additional guarantee to have been
/// successfully validated.
pub struct ValidatedForm<T: Validate>(pub T);

impl<T: Validate> ValidatedForm<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Validate> ops::Deref for ValidatedForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Validate> ops::DerefMut for ValidatedForm<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

// https://docs.rs/actix-web/2.0.0/src/actix_web/types/form.rs.html#112
impl<T> FromRequest for ValidatedForm<T>
where
    T: Validate + DeserializeOwned + 'static,
{
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = ValidatedFormConfig;

    #[inline]
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req2 = req.clone();
        let config = req
            .app_data::<Self::Config>()
            .map(|c| c.clone())
            .unwrap_or(Self::Config::default());

        UrlEncoded::new(req, payload)
            .limit(config.limit)
            .map_err(move |e| ValidatedFormError::Deserialization(e))
            .and_then(|c: T| match c.validate() {
                Ok(_) => future::ok(c),
                Err(e) => future::err(ValidatedFormError::Validation(e)),
            })
            .map_ok(ValidatedForm)
            .map_err(move |e| {
                if let Some(err) = config.error_handler {
                    (*err)(e, &req2)
                } else {
                    e.into()
                }
            })
            .boxed_local()
    }
}

impl<T: Validate + fmt::Debug> fmt::Debug for ValidatedForm<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Validate + fmt::Display> fmt::Display for ValidatedForm<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Configure the behaviour of the ValidatedForm extractor
///
/// # Usage
/// Add a `ValidatedFormConfig` to your actix app data
/// ```
/// .app_data(
///     ValidatedFormConfig::default().error_handler(|e, _| YourCustomErrorType::from(e).into())
/// )
/// ```
#[derive(Clone)]
pub struct ValidatedFormConfig {
    limit: usize,
    error_handler:
        Option<Rc<dyn Fn(ValidatedFormError<UrlencodedError>, &HttpRequest) -> actix_web::Error>>,
}

impl ValidatedFormConfig {
    /// Set the max size of payload. By default max size is 16Kb
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Sets a custom error handler to convert the error (arising from a form that failed to
    /// either deserialize or validate) into a different type
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(ValidatedFormError<UrlencodedError>, &HttpRequest) -> actix_web::Error + 'static,
    {
        self.error_handler = Some(Rc::new(f));
        self
    }
}

impl Default for ValidatedFormConfig {
    fn default() -> Self {
        ValidatedFormConfig {
            limit: 16384,
            error_handler: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{test, web, App, HttpResponse, Responder, ResponseError};
    use serde::{Deserialize, Serialize};
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate, Serialize)]
    pub struct ExampleForm {
        #[validate(length(min = 1, max = 5))]
        field: String,
    }

    async fn route(form: ValidatedForm<ExampleForm>) -> impl Responder {
        HttpResponse::Ok().json(&*form)
    }

    #[actix_rt::test]
    async fn test_valid() {
        let mut app = test::init_service(App::new().route("/", web::get().to(route))).await;
        let form = ExampleForm {
            field: "abc".to_string(),
        };
        let req = test::TestRequest::with_uri("/")
            .set_form(&form)
            .to_request();
        let resp: ExampleForm = test::read_response_json(&mut app, req).await;
        assert_eq!(resp.field, "abc");
    }

    #[derive(Debug)]
    struct Teapot;

    impl std::fmt::Display for Teapot {
        fn fmt(&self, _f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
            unimplemented!()
        }
    }

    impl ResponseError for Teapot {
        fn error_response(&self) -> HttpResponse {
            HttpResponse::build(StatusCode::IM_A_TEAPOT).finish()
        }
    }

    #[actix_rt::test]
    async fn test_invalid() {
        let mut app = test::init_service(
            App::new()
                .app_data(ValidatedFormConfig::default().error_handler(|_, _| Teapot {}.into()))
                .route("/", web::get().to(route)),
        )
        .await;
        let form = ExampleForm {
            field: "too long for validation".to_string(),
        };
        let req = test::TestRequest::with_uri("/")
            .set_form(&form)
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
    }
}
