use csync_misc::api::metadata::{GetMetadataRequest, Metadata, ServerState};
use csync_misc::api::user::User;
use csync_misc::api::{EmptyRequest, ListResponse, Response};
use log::{debug, error};

use crate::context::ServerContext;
use crate::register_handlers;

register_handlers!(get_metadata, get_state);

async fn get_metadata(
    mut req: GetMetadataRequest,
    op: User,
    ctx: &ServerContext,
) -> Response<ListResponse<Metadata>> {
    if !op.admin {
        req.owner = Some(op.name.clone());
    }
    debug!("Get metadata: {:?}", req);

    let result = ctx.db.with_transaction(|tx| {
        let items = tx.get_metadatas(req.clone())?;
        let count = tx.count_metadatas(req)?;
        Ok(ListResponse {
            items,
            total: count,
        })
    });

    match result {
        Ok(data) => Response::with_data(data),
        Err(e) => {
            error!("Failed to get metadata: {e:#}");
            Response::database_error()
        }
    }
}

async fn get_state(_req: EmptyRequest, _op: User, ctx: &ServerContext) -> Response<ServerState> {
    let rev = ctx.get_state();
    Response::with_data(rev)
}
