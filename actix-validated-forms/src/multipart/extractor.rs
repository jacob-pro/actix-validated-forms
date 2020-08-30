use super::{load, MultipartField, MultipartForm, MultipartLoaderConfig};
use actix_web::web::Payload;
use actix_web::{FromRequest, HttpRequest};
use futures::Future;
use std::convert::TryFrom;
use std::{fmt, ops};
use validator::Validate;

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
//
//impl<T> FromRequest for ValidatedMultipartForm<T>
//    where
//        T: Validate + TryFrom<MultipartForm> + 'static,
//{
//    type Error = actix_web::Error;
//    type Future = Box<dyn Future<Item=Self, Error=Self::Error>>;
//    type Config = MultipartLoaderConfig;
//
//    #[inline]
//    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
//        let req2 = req.clone();
//        let config = req.app_data::<MultipartLoaderConfig>()
//            .map(|c| c.clone())
//            .unwrap_or(MultipartLoaderConfig::default());
//
//        Box::new(
//
//        )
//    }
//}
