mod files;
mod images;
mod roles;
mod texts;
mod union;
mod users;

pub mod dispatch;

use crate::server::authn::AuthnUserInfo;
use crate::server::response::Response;
use crate::types::request::Query;

pub trait ResourceHandler: Send + Sync {
    fn put(&self, req: PutRequest, user: AuthnUserInfo) -> Response;
    fn list(&self, query: Query, json: bool, user: AuthnUserInfo) -> Response;
    fn get(&self, id: String, json: bool, user: AuthnUserInfo) -> Response;
    fn delete(&self, id: String, user: AuthnUserInfo) -> Response;
}

pub enum PutRequest {
    Binary(Option<String>, Vec<u8>),
    Json(String),
}

#[macro_export]
macro_rules! expect_json {
    ($req:expr) => {
        match $req {
            $crate::server::handlers::resources::PutRequest::Json(data) => {
                match serde_json::from_str(&data) {
                    Ok(obj) => obj,
                    Err(_) => {
                        return $crate::server::response::Response::bad_request(
                            "Invalid json pyaload".to_string(),
                        );
                    }
                }
            }
            _ => {
                return $crate::server::response::Response::bad_request(
                    "Expect json payload".to_string(),
                )
            }
        }
    };
}

#[macro_export]
macro_rules! expect_binary {
    ($req:expr) => {
        match $req {
            $crate::server::handlers::resources::PutRequest::Binary(metadata, data) => {
                (metadata, data)
            }
            _ => {
                return $crate::server::response::Response::bad_request(
                    "Expect binary payload".to_string(),
                )
            }
        }
    };
}