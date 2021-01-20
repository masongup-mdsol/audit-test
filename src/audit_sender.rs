use hyper::header::CONTENT_TYPE;
use hyper::{Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde_json::json;

use crate::audit_creator::Audit;
use mauth_client::MAuthInfo;

pub async fn send_audits() {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let uri: hyper::Uri = "https://maudit-sandbox.imedidata.net/v1/audits"
        .parse()
        .unwrap();
    let fake_audit = Audit::create_fake_audit();
    let audit_json = json!({ "audits": [fake_audit] });
    //println!("Going to write audit with body like: {}", &audit_json);
    let (json_body, body_digest) = MAuthInfo::build_body_with_digest(audit_json.to_string());
    let mut req = Request::new(json_body);
    *req.method_mut() = Method::POST;
    *req.uri_mut() = uri.clone();
    let headers = req.headers_mut();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    let mauth_info = MAuthInfo::from_default_file().expect("Failed trying to load mauth info");
    mauth_info.sign_request_v2(&mut req, &body_digest);
    match client.request(req).await {
        Err(err) => println!("Got error {}", err),
        Ok(mut response) => match mauth_info.validate_response(&mut response).await {
            Ok(resp_body) => println!(
                "Got validated response body {}",
                &String::from_utf8(resp_body).unwrap()
            ),
            Err(err) => println!("Error validating response: {:?}", err),
        },
    }
}

pub async fn dalton_test() {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let uri: hyper::Uri = "https://dalton-sandbox.imedidata.net/v1/privileges/show?operable_uri=com:mdsol:client_division_schemes:7e3afb67-848a-4ddb-982c-04119f962916&operation=read_client_division_schemes&operator_uri=com:mdsol:users:54e6254c-86ee-4599-b021-8f243454c90b"
        .parse()
        .unwrap();
    let mauth_info = MAuthInfo::from_default_file().expect("Failed trying to load mauth info");
    let (body, body_digest) = MAuthInfo::build_body_with_digest("".to_string());
    let mut req = Request::new(body);
    *req.method_mut() = Method::GET;
    *req.uri_mut() = uri.clone();
    let headers = req.headers_mut();
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    mauth_info.sign_request_v2(&mut req, &body_digest);
    match client.request(req).await {
        Err(err) => println!("Got error {}", err),
        Ok(mut response) => {
            if response.status().is_success() {
                match mauth_info.validate_response(&mut response).await {
                    Ok(resp_body) => println!(
                        "Got validated response body {}",
                        &String::from_utf8(resp_body).unwrap()
                    ),
                    Err(err) => println!("Error validating response: {:?}", err),
                }
            } else {
                println!(
                    "Got response status {}, not verifying",
                    response.status().as_str()
                );
            }
        }
    }
}
