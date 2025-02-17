use actix_web::HttpRequest;
use chrono::Local;
use csync_misc::types::healthz::HealthzResponse;

use crate::now::current_timestamp;
use crate::response::Response;

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
        let now = current_timestamp();
        let response = HealthzResponse {
            now,
            time_zone: offset,
            client_ip: req.connection_info().peer_addr().map(|a| a.to_string()),
            version: Some(env!("CSYNC_VERSION").to_string()),
        };
        Response::json(response)
    }
}
