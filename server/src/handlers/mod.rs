mod resources;

pub mod api;
pub mod healthz;
pub mod login;

use actix_web::HttpRequest;

use super::response::Response;

pub trait Handler: Send + Sync {
    fn handle(&self, path: &str, req: HttpRequest, body: Option<Vec<u8>>) -> Response;
}
