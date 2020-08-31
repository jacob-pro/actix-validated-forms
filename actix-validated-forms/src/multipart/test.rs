use super::*;
use actix_multipart::Multipart;
use actix_multipart_rfc7578::client::multipart;
use actix_web::{test, web, App, Error, HttpResponse};
use awc::Client;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::io::{Write, Read};

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    string: String,
    int: i32,
    file_content: String,
}

async fn route(payload: Multipart) -> Result<HttpResponse, Error> {
    let mut k = load(payload, MultipartLoadConfig::default()).await?;

    let mut data = String::new();
    let f: MultipartFile = MultipartType::get(&mut k, "file")?;
    println!("{:?}", f);
    f.file.reopen().unwrap().read_to_string(&mut data).unwrap();

    let r = Response {
        string: MultipartType::get(&mut k, "string")?,
        int: MultipartType::get(&mut k, "int")?,
        file_content: data
    };
    Ok(HttpResponse::Ok().json(r).into())
}

#[actix_rt::test]
async fn test() {
    let srv = test::start(|| App::new().route("/", web::post().to(route)));

    let mut form = multipart::Form::default();
    form.add_text("string", "Hello World");
    form.add_text("int", "69");

    let temp = NamedTempFile::new().unwrap();
    temp.as_file().write("File contents".as_bytes());
    form.add_file("file", temp.path());

    let mut response = Client::default()
        .post(srv.url("/"))
        .content_type(form.content_type())
        .send_body(multipart::Body::from(form))
        .await
        .unwrap();

    assert!(response.status().is_success());
    let res: Response = response.json().await.unwrap();
    assert_eq!(res.string, "Hello World");
    assert_eq!(res.int, 69);
    assert_eq!(res.file_content, "File contents");
}
