use std::fs;
use std::cell::RefCell;
use std::collections::HashMap;

use serde_json;
use hyper::{Client, Body, Method, Request, Response};
use hyper::rt::Future;
use hyper::header::{HeaderValue, CONTENT_TYPE};
use hyper_tls::HttpsConnector;
use futures::stream::Stream;
use tokio::runtime::Runtime;
use chrono::prelude::*;
use uuid::Uuid;
use sha2::{Sha512, Digest};
use hex;
use ring::signature::{RsaKeyPair, UnparsedPublicKey, RSA_PKCS1_SHA512, RSA_PKCS1_2048_8192_SHA512};
use ring::rand::{SystemRandom};
use base64;
use dirs;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;

const CONFIG_FILE: &str = ".mauth_config.yml";

pub fn send_audits() {
    let mut runtime = Runtime::new().expect("Unable to create a runtime");
    let https = HttpsConnector::new(4).unwrap();
    let client = Client::builder().build::<_, hyper::Body>(https);
    //let client = Client::new();
    let uri: hyper::Uri = "https://maudit-sandbox.imedidata.net/v1/audits".parse().unwrap();
    //let uri: hyper::Uri = "http://localhost:3000/v1/audits".parse().unwrap();
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

pub fn test_crypto() {
    let mut priv_key_path = dirs::home_dir().unwrap();
    priv_key_path.push(".mauth_key");
    let openssl_key = PKey::private_key_from_pem(&fs::read(&priv_key_path).unwrap()).unwrap();
    let ring_priv_key = RsaKeyPair::from_der(&openssl_key.private_key_to_der().unwrap()).unwrap();
    let mut signature = vec![0; ring_priv_key.public_modulus_len()];
    let message = "test string".as_bytes();
    ring_priv_key.sign(&RSA_PKCS1_SHA512, &SystemRandom::new(), &message, &mut signature).unwrap();

    let mut pub_key_path = dirs::home_dir().unwrap();
    pub_key_path.push(".mauth_key.pub");
    let pub_key_data = fs::read(&pub_key_path).unwrap();
    let pub_key = Rsa::public_key_from_pem(&pub_key_data).unwrap();
    let ring_pub_key = UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA512, pub_key.public_key_to_der_pkcs1().unwrap());
    match ring_pub_key.verify(&message, &signature) {
        Ok(()) => println!("Signature matches!"),
        Err(_) => println!("Failed to match signature"),
    }
}

struct MAuthInfo {
    app_id: Uuid,
    private_key: RsaKeyPair,
    mauth_uri_base: hyper::Uri,
    remote_key_store: RefCell<HashMap<Uuid, UnparsedPublicKey<bytes::Bytes>>>,
}

impl MAuthInfo {
    fn from_default_file() -> Result<MAuthInfo, String> {
        let mut home = dirs::home_dir().unwrap();
        home.push(CONFIG_FILE);
        let config_data = fs::read(&home).map_err(|_| "Couldn't open config file")?;
        let config = serde_yaml::from_slice::<serde_yaml::Value>(&config_data).unwrap();
        let section = config.get("common").ok_or("Invalid config file format")?;
        let uuid_str = section.get("app_uuid").and_then(|u| u.as_str()).ok_or("Invalid config file format")?;
        let mauth_server_uri_str = section.get("mauth_baseurl").and_then(|u| u.as_str())
            .ok_or("Invalid config file format")?;
        let mauth_server_version_str = section.get("mauth_api_version").and_then(|u| u.as_str())
            .ok_or("Invalid config file format")?;
        let full_uri: hyper::Uri = format!("{}/mauth/{}/security_tokens/", &mauth_server_uri_str, &mauth_server_version_str)
            .parse()
            .map_err(|_| "Invalid config file format")?;
        let pk_path_str = section.get("private_key_file").and_then(|u| u.as_str()).ok_or("Invalid config file format")?;

        let pk_data = fs::read(&pk_path_str).map_err(|_| "Couldn't open key file")?;
        let openssl_key = PKey::private_key_from_pem(&pk_data).map_err(|e| format!("OpenSSL Key Load Error: {}", e))?;
        let der_key_data = openssl_key.private_key_to_der().unwrap();
        Ok(MAuthInfo {
            app_id: Uuid::parse_str(uuid_str).map_err(|_| "UUID from config file was bad")?,
            mauth_uri_base: full_uri,
            remote_key_store: RefCell::new(HashMap::new()),
            private_key: RsaKeyPair::from_der(&der_key_data).map_err(|e| {
                println!("PK process error {:#?}", e);
                "Invalid private key"
            })?
        })
    }

    fn build_body_with_digest(body: String) -> (Body, String) {
        let mut hasher = Sha512::default();
        hasher.input(body.as_bytes());
        (Body::from(body.clone()), hex::encode(hasher.result()))
    }

    fn sign_request(&self, req: &mut Request<Body>, body_digest: String) {
        let timestamp_str = Utc::now().timestamp().to_string();
        let string_to_sign = format!("{}\n{}\n{}\n{}\n{}\n",
            req.method(),
            req.uri().path(),
            &body_digest,
            &self.app_id,
            &timestamp_str,
        );

        let mut signature = vec![0; self.private_key.public_modulus_len()];
        self.private_key.sign(&RSA_PKCS1_SHA512, &SystemRandom::new(), string_to_sign.as_bytes(), &mut signature).unwrap();
        let signature = format!("MWSV2 {}:{};", self.app_id, base64::encode(&signature));

        let headers = req.headers_mut();
        headers.insert("MCC-Time", HeaderValue::from_str(&timestamp_str).unwrap());
        headers.insert("MCC-Authentication", HeaderValue::from_str(&signature).unwrap());
    }

    fn validate_response(&self, response: Response<Body>, mut runtime: &mut Runtime) -> Result<String, MAuthValidationError> {
        let (parts, body) = response.into_parts();
        let resp_headers = parts.headers;

        //retrieve and validate timestamp
        let ts_header = resp_headers.get("MCC-Time").ok_or(MAuthValidationError::NoTime)?;
        let ts_str = ts_header.to_str().map_err(|_| MAuthValidationError::InvalidTime)?;
        let ts_num: i64 = ts_str.parse().map_err(|_| MAuthValidationError::InvalidTime)?;
        let ts_diff = ts_num - Utc::now().timestamp();
        if ts_diff > 300 || ts_diff < -300 {
            return Err(MAuthValidationError::InvalidTime);
        }

        //retrieve and parse auth string
        let sig_header = resp_headers.get("MCC-Authentication").ok_or(MAuthValidationError::NoSig)?;
        let header_pattern = vec![' ', ':', ';'];
        let mut header_split = sig_header.to_str()
            .map_err(|_| MAuthValidationError::InvalidTime)?
            .split(header_pattern.as_slice());

        let start_str = header_split.nth(0).ok_or(MAuthValidationError::InvalidSignature)?;
        if start_str != "MWSV2" {
            return Err(MAuthValidationError::InvalidSignature);
        }
        let host_uuid_str = header_split.nth(0).ok_or(MAuthValidationError::InvalidSignature)?;
        let host_app_uuid = Uuid::parse_str(host_uuid_str).map_err(|_| MAuthValidationError::InvalidSignature)?;
        let signature_encoded_string = header_split.nth(0).ok_or(MAuthValidationError::InvalidSignature)?;
        let raw_signature: Vec<u8> = base64::decode(&signature_encoded_string)
            .map_err(|_| MAuthValidationError::InvalidSignature)?;

        //Compute response signing string
        let body_raw: Vec<u8> = body.collect().wait().map_err(|_| MAuthValidationError::ResponseProblem)?
            .into_iter()
            .flat_map(|chunk| chunk.into_bytes())
            .collect();
        let mut hasher = Sha512::default();
        hasher.input(&body_raw);
        let string_to_sign = format!("{}\n{}\n{}\n{}",
            &parts.status.as_u16(),
            hex::encode(hasher.result()),
            &host_app_uuid,
            &ts_str,
        );
        //Well at least I can confirm that the signing string is exactly the same

        match self.get_app_pub_key(&host_app_uuid, &mut runtime) {
            None => return Err(MAuthValidationError::KeyUnavailable),
            Some(pub_key) => {
                match pub_key.verify(&string_to_sign.as_bytes(), &raw_signature) {
                    Ok(()) => String::from_utf8(body_raw).map_err(|_| MAuthValidationError::InvalidBody),
                    Err(_) => Err(MAuthValidationError::SignatureVerifyFailure),
                }
            }
        }
    }

    fn get_app_pub_key(&self, app_uuid: &Uuid, runtime: &mut Runtime) -> Option<UnparsedPublicKey<bytes::Bytes>> {
        let mut key_store = self.remote_key_store.borrow_mut();
        if let Some(pub_key) = key_store.get(&app_uuid) {
            return Some(pub_key.clone());
        }
        let https = HttpsConnector::new(4).unwrap();
        let client = Client::builder().build::<_, hyper::Body>(https);
        let (get_body, body_digest) = MAuthInfo::build_body_with_digest("".to_string());
        let mut req = Request::new(get_body);
        *req.method_mut() = Method::GET;
        let mut uri_parts = self.mauth_uri_base.clone().into_parts();
        let mut path_str: String = uri_parts.path_and_query.take().unwrap().as_str().to_string();
        path_str.push_str(&format!("{}", &app_uuid));
        uri_parts.path_and_query = Some(path_str.parse().unwrap());
        let uri = hyper::Uri::from_parts(uri_parts).unwrap();
        *req.uri_mut() = uri;
        self.sign_request(&mut req, body_digest);
        let mauth_response = runtime.block_on(client.request(req));
        match mauth_response {
            Err(_) => None,
            Ok(response) => {
                let response_str = String::from_utf8(response.into_body().collect().wait().unwrap()
                    .into_iter()
                    .flat_map(|chunk| chunk.into_bytes())
                    .collect()
                    ).unwrap();
                let response_obj: serde_json::Value = serde_json::from_str(&response_str).unwrap();
                let pub_key_str = response_obj.pointer("/security_token/public_key_str")
                    .and_then(|s| s.as_str())
                    .unwrap();
                let pub_key = Rsa::public_key_from_pem(&pub_key_str.as_bytes()).unwrap();
                let ring_key = UnparsedPublicKey::new(
                    &RSA_PKCS1_2048_8192_SHA512,
                    bytes::Bytes::from(pub_key.public_key_to_der_pkcs1().unwrap())
                );
                key_store.insert(app_uuid.clone(), ring_key.clone());
                Some(ring_key)
            }
        }
    }
}

#[derive(Debug)]
enum MAuthValidationError {
    InvalidTime,
    InvalidSignature,
    NoTime,
    NoSig,
    ResponseProblem,
    InvalidBody,
    KeyUnavailable,
    SignatureVerifyFailure,
}
