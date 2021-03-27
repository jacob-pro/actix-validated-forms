use crate::error::ValidatedFormError;
use actix_web::dev::Payload;
use actix_web::error::QueryPayloadError;
use actix_web::{FromRequest, HttpRequest};
use futures::future::{err, ok, Ready};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::{fmt, ops};
use validator::Validate;

/// Validated extractor for a Url Encoded HTTP Query String
///
/// # Example
/// First define a structure to represent the query that implements `serde::Deserialize` and
/// `validator::Validate` traits.
/// ```
/// #[derive(Deserialize, Validate)]
/// struct ExampleQuery {
///     #[validate(range(min = 1, max = 100))]
///     limit: i64,
///     offset: i64,
///     search: Option<String>,
/// }
/// ```
/// Use the extractor in your route:
/// ```
/// async fn route(
///     query: ValidatedQuery<ExampleQuery>,
/// ) -> impl Responder { ... }
/// ```
/// Just like the `actix_web::web::Query` when the body of route is executed `query` can be
/// dereferenced to an `ExampleQuery`, however it has the additional guarantee to have been
/// successfully validated.
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
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ValidatedQueryConfig;

    #[inline]
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let config = req
            .app_data::<Self::Config>()
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
            .map(|val| ok(ValidatedQuery(val)))
            .unwrap_or_else(move |e| {
                let e = if let Some(error_handler) = config.error_handler {
                    (error_handler)(e, req)
                } else {
                    e.into()
                };
                err(e)
            })
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

/// Configure the behaviour of the ValidatedQuery extractor
///
/// # Usage
/// Add a `ValidatedQueryConfig` to your actix app data
/// ```
/// .app_data(
///     ValidatedQueryConfig::default().error_handler(|e, _| YourCustomErrorType::from(e).into())
/// )
/// ```
#[derive(Clone)]
pub struct ValidatedQueryConfig {
    error_handler: Option<
        Arc<
            dyn Fn(ValidatedFormError<QueryPayloadError>, &HttpRequest) -> actix_web::Error
                + Send
                + Sync,
        >,
    >,
}

impl ValidatedQueryConfig {
    /// Sets a custom error handler to convert the error (arising from a query that failed to
    /// either deserialize or validate) into a different type
    pub fn error_handler<F>(mut self, f: F) -> Self
    where
        F: Fn(ValidatedFormError<QueryPayloadError>, &HttpRequest) -> actix_web::Error
            + Send
            + Sync
            + 'static,
    {
        self.error_handler = Some(Arc::new(f));
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{test, web, App, HttpResponse, Responder, ResponseError};
    use serde::{Deserialize, Serialize};
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate, Serialize)]
    pub struct ExampleQuery {
        #[validate(range(min = 1, max = 100))]
        limit: i64,
        offset: i64,
        search: Option<String>,
    }

    async fn route(query: ValidatedQuery<ExampleQuery>) -> impl Responder {
        HttpResponse::Ok().json(&*query)
    }

    #[actix_rt::test]
    async fn test_valid() {
        let mut app = test::init_service(App::new().route("/", web::get().to(route))).await;
        let req = test::TestRequest::with_uri("/?limit=20&offset=40&search=hello").to_request();
        let resp: ExampleQuery = test::read_response_json(&mut app, req).await;
        assert_eq!(resp.limit, 20);
        assert_eq!(resp.offset, 40);
        assert_eq!(resp.search, Some("hello".to_string()));
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
                .app_data(ValidatedQueryConfig::default().error_handler(|_, _| Teapot {}.into()))
                .route("/", web::get().to(route)),
        )
        .await;
        let req = test::TestRequest::with_uri("/?limit=9999&offset=4000").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
    }
}
