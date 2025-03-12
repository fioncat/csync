use std::sync::Arc;

use actix_web::web::{Bytes, Data};
use actix_web::{HttpRequest, HttpResponse};
use chrono::Utc;
use csync_misc::api::{HealthResponse, Response};

use crate::context::ServerContext;

pub async fn get_healthz_handler(
    _req: HttpRequest,
    _body: Option<Bytes>,
    _ctx: Data<Arc<ServerContext>>,
) -> HttpResponse {
    let now = Utc::now().timestamp() as u64;
    let resp = Response::with_data(HealthResponse {
        version: env!("CSYNC_VERSION").to_string(),
        timestamp: now,
    });
    HttpResponse::Ok().json(resp)
}
