use actix_web::HttpRequest;
use chrono::Local;

use crate::server::response::Response;
use crate::types::healthz::HealthzResponse;

use super::Handler;

pub struct HealthzHandler;

impl HealthzHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Handler for HealthzHandler {
    fn handle(&self, _path: &str, req: HttpRequest, _body: Option<Vec<u8>>) -> Response {
        let local = Local::now();
        let offset = format!("{}", local.offset());
        let now = local.timestamp() as u64;
        let response = HealthzResponse {
            now,
            time_zone: offset,
            client_ip: req.connection_info().peer_addr().map(|a| a.to_string()),
            version: Some(env!("CSYNC_VERSION").to_string()),
        };
        Response::json(response)
    }
}
