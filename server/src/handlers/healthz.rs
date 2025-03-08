use chrono::Utc;
use csync_misc::api::user::User;
use csync_misc::api::{EmptyRequest, HealthResponse, Response};

use crate::context::ServerContext;
use crate::register_handlers;

register_handlers!(get_healthz);

async fn get_healthz(
    _req: EmptyRequest,
    _op: User,
    _sc: &ServerContext,
) -> Response<HealthResponse> {
    let now = Utc::now().timestamp() as u64;
    Response::with_data(HealthResponse {
        version: env!("CSYNC_VERSION").to_string(),
        timestamp: now,
    })
}
