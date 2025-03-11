use csync_misc::api::metadata::{GetMetadataRequest, Metadata, Revision};
use csync_misc::api::user::User;
use csync_misc::api::{EmptyRequest, ListResponse, Response};
use log::{debug, error};

use crate::context::ServerContext;
use crate::register_handlers;

register_handlers!(get_metadata, get_revision);

async fn get_metadata(
    mut req: GetMetadataRequest,
    op: User,
    sc: &ServerContext,
) -> Response<ListResponse<Metadata>> {
    if !op.admin {
        req.owner = Some(op.name.clone());
    }
    debug!("Get metadata: {:?}", req);

    let result = sc.db.with_transaction(|tx| {
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

async fn get_revision(_req: EmptyRequest, _op: User, ctx: &ServerContext) -> Response<Revision> {
    let rev = ctx.get_revision();
    Response::with_data(rev)
}
