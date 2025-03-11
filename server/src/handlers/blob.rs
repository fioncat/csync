use chrono::Utc;
use csync_misc::api::blob::{Blob, GetBlobRequest, PatchBlobRequest};
use csync_misc::api::metadata::{BlobType, GetMetadataRequest, Metadata};
use csync_misc::api::user::User;
use csync_misc::api::Response;
use csync_misc::{code, humanize};
use log::{debug, error};
use unicode_width::UnicodeWidthChar;

use crate::context::ServerContext;
use crate::db::types::{CreateBlobParams, PatchBlobParams};
use crate::register_handlers;

register_handlers!(put_blob, patch_blob, get_blob, delete_blob);

async fn put_blob(req: Blob, op: User, ctx: &ServerContext) -> Response<()> {
    let sha256 = code::sha256(&req.data);
    if sha256 != req.sha256 {
        return Response::bad_request("data sha256 mismatch");
    }
    debug!(
        "Put blob, type: {:?}, sha256: {}, size: {}",
        req.blob_type,
        sha256,
        req.data.len()
    );

    let result = ctx.db.with_transaction(|tx| {
        let sha256_query = GetMetadataRequest {
            sha256: Some(sha256.clone()),
            ..Default::default()
        };
        let duplicates = tx.get_metadatas(sha256_query)?;
        if !duplicates.is_empty() {
            debug!(
                "Found duplicate blobs with sha256 {}: {:?}, delete them",
                sha256, duplicates
            );
            let ids: Vec<_> = duplicates.iter().map(|m| m.id).collect();
            tx.delete_blobs(ids)?;
        }

        let summary = get_summary(&req, ctx.cfg.truncate_text_width);
        let update_time = Utc::now().timestamp() as u64;
        let recycle_time = update_time + ctx.cfg.recycle_seconds;
        let blob_type = req.blob_type;
        let size = req.data.len() as u64;
        let params = CreateBlobParams {
            blob: req,
            summary: summary.clone(),
            owner: op.name.clone(),
            update_time,
            recycle_time,
        };

        let id = tx.create_blob(params)?;
        let latest = Metadata {
            id,
            pin: false,
            blob_type,
            blob_sha256: sha256,
            blob_size: size,
            summary,
            owner: op.name.clone(),
            recycle_time,
            update_time,
        };

        Ok(latest)
    });

    match result {
        Ok(latest) => {
            debug!("Create blob done, update revision latest: {latest:?}");
            ctx.update_latest(latest);
            Response::ok()
        }
        Err(e) => {
            error!("Failed to create blob: {e:#}");
            Response::database_error()
        }
    }
}

async fn patch_blob(req: PatchBlobRequest, op: User, ctx: &ServerContext) -> Response<()> {
    debug!("Patch blob: {req:?}");
    let result = ctx.db.with_transaction(|tx| {
        if !tx.has_blob(req.id)? {
            return Ok(false);
        }

        if !op.admin {
            let metadata = tx.get_metadata(req.id)?;
            if metadata.owner != op.name {
                debug!(
                    "User {} try to patch blob {} owned by {}",
                    op.name, req.id, metadata.owner
                );
                return Ok(false);
            }
        }

        let now = Utc::now().timestamp() as u64;
        let params = PatchBlobParams {
            patch: req,
            update_time: now,
            recycle_time: now + ctx.cfg.recycle_seconds,
        };

        tx.update_blob(params)?;

        Ok(true)
    });

    match result {
        Ok(true) => {
            debug!("Patch blob done, growing revision");
            ctx.grow_revision();
            Response::ok()
        }
        Ok(false) => Response::resource_not_found(),
        Err(e) => {
            error!("Failed to update blob: {e:#}");
            Response::database_error()
        }
    }
}

async fn get_blob(req: GetBlobRequest, op: User, sc: &ServerContext) -> Response<()> {
    debug!("Get blob: {req:?}");

    let result = sc.db.with_transaction(|tx| {
        if !tx.has_blob(req.id)? {
            return Ok(None);
        }

        if !op.admin {
            let metadata = tx.get_metadata(req.id)?;
            if metadata.owner != op.name {
                debug!(
                    "User {} try to download blob {} owned by {}",
                    op.name, req.id, metadata.owner
                );
                return Ok(None);
            }
        }

        let blob = tx.get_blob(req.id)?;
        Ok(Some(blob))
    });

    match result {
        Ok(Some(blob)) => Response::with_blob(blob),
        Ok(None) => Response::resource_not_found(),
        Err(e) => {
            error!("Failed to get blob: {e:#}");
            Response::database_error()
        }
    }
}

async fn delete_blob(req: GetBlobRequest, op: User, ctx: &ServerContext) -> Response<()> {
    debug!("Delete blob: {req:?}");

    let result = ctx.db.with_transaction(|tx| {
        if !tx.has_blob(req.id)? {
            return Ok(false);
        }

        let metadata = tx.get_metadata(req.id)?;
        if !op.admin && metadata.owner != op.name {
            debug!(
                "User {} try to delete blob {} owned by {}",
                op.name, req.id, metadata.owner
            );
            return Ok(false);
        }

        tx.delete_blob(req.id)?;
        Ok(true)
    });

    match result {
        Ok(true) => {
            debug!("Delete blob done, growing revision");
            ctx.grow_revision();
            Response::ok()
        }
        Ok(false) => Response::resource_not_found(),
        Err(e) => {
            error!("Failed to delete blob: {e:#}");
            Response::database_error()
        }
    }
}

fn get_summary(blob: &Blob, text_width: usize) -> String {
    match blob.blob_type {
        BlobType::Text => {
            let text = match String::from_utf8(blob.data.clone()) {
                Ok(text) => text,
                Err(_) => return String::new(),
            };
            truncate_text(text, text_width)
        }
        BlobType::Image => {
            let size = humanize::human_bytes(blob.data.len() as u64);
            format!("<PNG Image, {size}>")
        }
        BlobType::File => {
            let size = humanize::human_bytes(blob.data.len() as u64);
            let file_name = blob.file_name.clone().unwrap_or_default();
            format!("<File, {file_name}, {size}>")
        }
    }
}

fn truncate_text(text: String, width: usize) -> String {
    let text = text.replace("\n", " ");

    let mut current_width = 0;
    let mut result = String::new();

    for c in text.chars() {
        let char_width = c.width_cjk().unwrap_or(1);

        if current_width + char_width > width {
            break;
        }

        result.push(c);
        current_width += char_width;
    }

    if result.len() < text.len() {
        result.push_str("...");
    }

    result
}
