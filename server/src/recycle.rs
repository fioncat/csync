use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use csync_misc::api::metadata::{Event, EventType, GetMetadataRequest};
use log::{debug, error, info};

use crate::context::ServerContext;

pub async fn start_recycle(ctx: Arc<ServerContext>) {
    let intv_secs = ctx.cfg.recycle_seconds;
    info!("Recycle loop starting, interval: {}s", intv_secs);
    let mut tk = tokio::time::interval(Duration::from_secs(intv_secs));
    loop {
        let _ = tk.tick().await;

        let result = ctx.db.with_transaction(|tx| {
            let now = Utc::now().timestamp() as u64;

            let req = GetMetadataRequest {
                recycle_before: Some(now),
                ..Default::default()
            };
            let count = tx.count_metadatas(req.clone())?;
            if count > 0 {
                let metadatas = tx.get_metadatas(req)?;
                let ids: Vec<_> = metadatas.iter().map(|m| m.id).collect();

                info!("Begin to recycle expired blobs: {:?}", ids);
                let count = tx.delete_blobs(ids)?;
                info!("Recycled {count} blobs");

                return Ok(Some(metadatas));
            }

            Ok(None)
        });

        match result {
            Ok(None) => {}
            Ok(Some(deleted)) => {
                debug!("Notify recycle items event: {:?}", deleted);
                let event = Event {
                    event_type: EventType::Delete,
                    items: deleted,
                };
                ctx.notify_event(event).await;
            }
            Err(e) => {
                error!("Failed to recycle blobs: {:#}", e);
            }
        }
    }
}
