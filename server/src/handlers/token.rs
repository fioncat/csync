use chrono::Utc;
use csync_misc::api::user::{TokenResponse, User};
use csync_misc::api::{EmptyRequest, Response};
use log::{debug, error};

use crate::context::ServerContext;
use crate::register_handlers;

register_handlers!(get_token);

async fn get_token(_req: EmptyRequest, op: User, ctx: &ServerContext) -> Response<TokenResponse> {
    debug!("Generate token for user: {op:?}");
    let now = Utc::now().timestamp() as u64;
    match ctx.jwt_generator.generate_token(op, now) {
        Ok(token) => {
            debug!("Token generated: {token:?}");
            Response::with_data(token)
        }
        Err(e) => {
            error!("Failed to generate token: {e:#}");
            Response::internal_server_error("failed to generate token")
        }
    }
}
