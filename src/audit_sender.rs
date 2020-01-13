
use hyper::{Client, Method, Request};
use hyper::header::CONTENT_TYPE;
use hyper_tls::HttpsConnector;
use tokio::runtime::Runtime;

use crate::mauth_client::MAuthInfo;

pub fn send_audits() {
    let mut runtime = Runtime::new().expect("Unable to create a runtime");
    let https = HttpsConnector::new(4).unwrap();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let uri: hyper::Uri = "https://maudit-sandbox.imedidata.net/v1/audits".parse().unwrap();
    let (json_body, body_digest) = MAuthInfo::build_body_with_digest(r#"{"audits": [] }"#.to_string());
    let mut req = Request::new(json_body);
    *req.method_mut() = Method::POST;
    *req.uri_mut() = uri.clone();
    let headers = req.headers_mut();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    let mauth_info = MAuthInfo::from_default_file().expect("Failed trying to load mauth info");
    mauth_info.sign_request(&mut req, body_digest);
    match runtime.block_on(client.request(req)) {
        Err(err) => println!("Got error {}", err),
        Ok(response) => {
            match mauth_info.validate_response(response, &mut runtime) {
                Ok(resp_body) => println!("Got validated response body {}", &resp_body),
                Err(err) => println!("Error validating response: {:?}", err),
            }
        },
    }
}


