use hyper::header::CONTENT_TYPE;
use hyper::{Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde_json::json;

use crate::audit_creator::Audit;
use crate::mauth_client::MAuthInfo;

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
    let mauth_info = MAuthInfo::from_default_file().await.expect("Failed trying to load mauth info");
    mauth_info.sign_request_v2(&mut req, body_digest);
    match client.request(req).await {
        Err(err) => println!("Got error {}", err),
        Ok(response) => match mauth_info.validate_response_v2(response).await {
            Ok(resp_body) => println!("Got validated response body {}", &resp_body),
            Err(err) => println!("Error validating response: {:?}", err),
        },
    }
}
