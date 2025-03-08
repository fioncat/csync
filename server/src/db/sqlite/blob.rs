use anyhow::{bail, Result};
use csync_misc::api::blob::Blob;
use csync_misc::api::metadata::{BlobType, GetMetadataRequest, Metadata};
use csync_misc::api::Value;
use log::debug;
use rusqlite::types::Value as DbValue;
use rusqlite::{params, params_from_iter, Connection, Transaction};

use crate::db::sql::{Select, Update};
use crate::db::types::{CreateBlobParams, PatchBlobParams};

use super::convert_values;

const CREATE_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS blob (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data BLOB NOT NULL,
    data_type INTEGER NOT NULL,
    summary TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL,
    pin INTEGER NOT NULL,
    file_name TEXT DEFAULT NULL,
    file_mode INTEGER DEFAULT NULL,
    owner TEXT NOT NULL,
    update_time INTEGER NOT NULL,
    recycle_time INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_blob_owner ON blob(owner);
CREATE INDEX IF NOT EXISTS idx_blob_sha256 ON blob(sha256);
CREATE INDEX IF NOT EXISTS idx_blob_summary ON blob(summary);
CREATE INDEX IF NOT EXISTS idx_blob_update_time ON blob(update_time);
CREATE INDEX IF NOT EXISTS idx_blob_recycle_time ON blob(recycle_time);
"#;

pub fn create_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLE_SQL)?;
    Ok(())
}

pub fn create(tx: &Transaction, params: CreateBlobParams) -> Result<u64> {
    let sql = r#"
    INSERT INTO blob (data, data_type, summary, sha256, size, pin, file_name, file_mode, owner, update_time, recycle_time)
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#;
    debug!("Database create_blob");
    tx.execute(
        sql,
        params![
            params.blob.data,
            params.blob.blob_type.to_code(),
            params.summary,
            params.blob.sha256,
            params.blob.data.len() as u64,
            false,
            params.blob.file_name,
            params.blob.file_mode,
            params.owner,
            params.update_time,
            params.recycle_time,
        ],
    )?;

    let id = tx.last_insert_rowid() as u64;
    Ok(id)
}

pub fn update(tx: &Transaction, params: PatchBlobParams) -> Result<()> {
    let PatchBlobParams {
        patch,
        update_time,
        recycle_time,
    } = params;
    let mut update = Update::new("blob");
    if let Some(pin) = patch.pin {
        update.add_field("pin", Value::Bool(pin));
        if pin {
            update.add_field("recycle_time", Value::Integer(0));
        } else {
            update.add_field("recycle_time", Value::Integer(recycle_time));
        }
    }
    update.add_field("update_time", Value::Integer(update_time));

    update.add_where("id = ?", Value::Integer(patch.id));

    let (sql, values) = update.build();
    if sql.is_empty() {
        return Ok(());
    }
    let values = convert_values(values);

    debug!("Database update_blob: {sql}, {values:?}");
    tx.execute(&sql, params_from_iter(values.iter()))?;

    Ok(())
}

pub fn delete(tx: &Transaction, id: u64) -> Result<()> {
    let sql = "DELETE FROM blob WHERE id = ?";
    debug!("Database delete_blob: {sql}, {id}");
    tx.execute(sql, params![id])?;
    Ok(())
}

pub fn delete_batch(tx: &Transaction, ids: Vec<u64>) -> Result<u64> {
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("DELETE FROM blob WHERE id IN ({})", placeholders);
    debug!("Database delete_blob_batch: {sql}, {ids:?}");
    let count = tx.execute(&sql, params_from_iter(ids.iter()))?;
    Ok(count as u64)
}

pub fn get(tx: &Transaction, id: u64) -> Result<Blob> {
    let mut select = Select::new(
        vec!["data", "sha256", "data_type", "file_name", "file_mode"],
        "blob",
    );
    select.add_where("id = ?", Value::Integer(id));

    let (sql, values) = select.build();
    let values = convert_values(values);

    let mut stmt = tx.prepare(&sql)?;
    let blob = stmt.query_row(params_from_iter(values), |row| {
        Ok(Blob {
            data: row.get(0)?,
            sha256: row.get(1)?,
            blob_type: parse_blob_type(row.get(2)?),
            file_name: row.get(3)?,
            file_mode: row.get(4)?,
        })
    })?;

    Ok(blob)
}

pub fn has(tx: &Transaction, id: u64) -> Result<bool> {
    let req = GetMetadataRequest {
        id: Some(id),
        ..Default::default()
    };
    debug!("Database has_blob: {id}");
    let count = count_metadatas(tx, req)?;
    Ok(count > 0)
}

pub fn get_metadata(tx: &Transaction, id: u64) -> Result<Metadata> {
    let req = GetMetadataRequest {
        id: Some(id),
        ..Default::default()
    };
    debug!("Database get_metadata_by_id: {id}");
    let mut metadatas = get_metadatas(tx, req)?;
    if metadatas.is_empty() {
        bail!("metadata {id} not found");
    }
    if metadatas.len() > 1 {
        bail!("multiple metadata {id} found");
    }

    Ok(metadatas.remove(0))
}

pub fn count_metadatas(tx: &Transaction, req: GetMetadataRequest) -> Result<u64> {
    let (sql, values) = build_select_sql(true, req);
    debug!("Database query count_metadata: {sql}, {values:?}");

    let mut stmt = tx.prepare(&sql)?;

    let count: i64 = stmt.query_row(params_from_iter(values), |row| row.get(0))?;

    Ok(count as u64)
}

pub fn get_metadatas(tx: &Transaction, req: GetMetadataRequest) -> Result<Vec<Metadata>> {
    let (sql, values) = build_select_sql(false, req);
    debug!("Database query get_metadata: {sql}, {values:?}");

    let mut stmt = tx.prepare(&sql)?;

    let metadatas = stmt
        .query_map(params_from_iter(values), |row| {
            Ok(Metadata {
                id: row.get(0)?,
                blob_type: parse_blob_type(row.get(1)?),
                summary: row.get(2)?,
                blob_sha256: row.get(3)?,
                blob_size: row.get(4)?,
                pin: row.get(5)?,
                owner: row.get(6)?,
                update_time: row.get(7)?,
                recycle_time: row.get(8)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    Ok(metadatas)
}

fn build_select_sql(count: bool, req: GetMetadataRequest) -> (String, Vec<DbValue>) {
    let mut select = if count {
        Select::count("blob")
    } else {
        Select::new(
            vec![
                "id",
                "data_type",
                "summary",
                "sha256",
                "size",
                "pin",
                "owner",
                "update_time",
                "recycle_time",
            ],
            "blob",
        )
    };

    if let Some(id) = req.id {
        select.add_where("id = ?", Value::Integer(id));
    }

    if let Some(owner) = req.owner {
        select.add_where("owner = ?", Value::Text(owner));
    }

    if let Some(sha256) = req.sha256 {
        select.add_where("sha256 = ?", Value::Text(sha256));
    }

    if let Some(recycle_before) = req.recycle_before {
        select.add_where(
            "recycle_time > 0 AND recycle_time < ?",
            Value::Integer(recycle_before),
        );
    }

    select.set_query(req.query, "summary");

    select.add_order_by("pin DESC");
    select.add_order_by("update_time DESC");

    let (sql, values) = select.build();
    let values = convert_values(values);

    (sql, values)
}

fn parse_blob_type(code: u32) -> BlobType {
    match BlobType::parse_code(code) {
        Ok(blob_type) => blob_type,
        Err(_) => BlobType::Text,
    }
}
