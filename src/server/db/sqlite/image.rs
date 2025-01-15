use anyhow::Result;
use chrono::Local;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Transaction};

use crate::server::db::ImageRecord;
use crate::types::request::Query;

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS image (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    data BLOB NOT NULL,
    hash TEXT NOT NULL,
    size INTEGER NOT NULL,
    owner TEXT NOT NULL,
    create_time INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_image_owner ON image(owner);
CREATE INDEX IF NOT EXISTS idx_image_hash ON image(hash);
CREATE INDEX IF NOT EXISTS idx_image_create_time ON image(create_time);
"#;

pub fn create_image_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}

pub fn create_image(tx: &Transaction, mut image: ImageRecord) -> Result<ImageRecord> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO image (data, hash, size, owner, create_time) VALUES (?, ?, ?, ?, ?)",
        params![image.data, image.hash, image.size, image.owner, now],
    )?;
    let id = tx.last_insert_rowid() as u64;
    image.id = id;
    image.create_time = now;
    Ok(image)
}

pub fn is_image_exists(tx: &Transaction, id: u64, owner: Option<&str>) -> Result<bool> {
    let sql = if owner.is_some() {
        "SELECT COUNT(*) FROM image WHERE id = ? AND owner = ?"
    } else {
        "SELECT COUNT(*) FROM image WHERE id = ?"
    };

    let params = if let Some(owner) = owner {
        vec![Value::Integer(id as i64), Value::Text(String::from(owner))]
    } else {
        vec![Value::Integer(id as i64)]
    };

    let mut stmt = tx.prepare(sql)?;
    let count: i64 = stmt.query_row(params_from_iter(params), |row| row.get(0))?;
    Ok(count > 0)
}

pub fn get_image(
    tx: &Transaction,
    id: u64,
    owner: Option<&str>,
    simple: bool,
) -> Result<ImageRecord> {
    let sql = if simple {
        "SELECT id, hash, size, owner, create_time FROM image WHERE id = ?"
    } else {
        "SELECT id, data, hash, size, owner, create_time FROM image WHERE id = ?"
    };

    let mut sql = String::from(sql);
    let params = if let Some(owner) = owner {
        sql.push_str(" AND owner = ?");
        vec![Value::Integer(id as i64), Value::Text(String::from(owner))]
    } else {
        vec![Value::Integer(id as i64)]
    };

    let mut stmt = tx.prepare(&sql)?;
    let image = stmt.query_row(params_from_iter(params), |row| {
        let record = if simple {
            ImageRecord {
                id: row.get(0)?,
                data: Vec::new(),
                hash: row.get(1)?,
                size: row.get(2)?,
                owner: row.get(3)?,
                create_time: row.get(4)?,
            }
        } else {
            ImageRecord {
                id: row.get(0)?,
                data: row.get(1)?,
                hash: row.get(2)?,
                size: row.get(3)?,
                owner: row.get(4)?,
                create_time: row.get(5)?,
            }
        };
        Ok(record)
    })?;
    Ok(image)
}

pub fn get_latest_image(
    tx: &Transaction,
    owner: Option<&str>,
    simple: bool,
) -> Result<ImageRecord> {
    let sql = if simple {
        "SELECT id, hash, size, owner, create_time FROM image"
    } else {
        "SELECT id, data, hash, size, owner, create_time FROM image"
    };
    let mut sql = String::from(sql);

    let params = if let Some(owner) = owner {
        sql.push_str(" WHERE owner = ?");
        vec![owner]
    } else {
        vec![]
    };
    sql.push_str(" ORDER BY id DESC LIMIT 1");

    let mut stmt = tx.prepare(&sql)?;
    let image = stmt.query_row(params_from_iter(params), |row| {
        let record = if simple {
            ImageRecord {
                id: row.get(0)?,
                data: Vec::new(),
                hash: row.get(1)?,
                size: row.get(2)?,
                owner: row.get(3)?,
                create_time: row.get(4)?,
            }
        } else {
            ImageRecord {
                id: row.get(0)?,
                data: row.get(1)?,
                hash: row.get(2)?,
                size: row.get(3)?,
                owner: row.get(4)?,
                create_time: row.get(5)?,
            }
        };
        Ok(record)
    })?;
    Ok(image)
}

pub fn list_images(tx: &Transaction, query: Query) -> Result<Vec<ImageRecord>> {
    let where_clause = query.generate_where("owner", "create_time");
    let limit_clause = query.generate_limit();
    let params = query.params();

    let sql = format!("SELECT id, hash, size, owner, create_time FROM image {where_clause} ORDER BY id DESC {limit_clause}");

    let mut stmt = tx.prepare(&sql)?;
    let images = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            Ok(ImageRecord {
                id: row.get(0)?,
                data: Vec::new(),
                hash: row.get(1)?,
                size: row.get(2)?,
                owner: row.get(3)?,
                create_time: row.get(4)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    Ok(images)
}

pub fn count_images(tx: &Transaction, owner: Option<&str>) -> Result<usize> {
    let mut sql = String::from("SELECT COUNT(*) FROM image");
    let params = if let Some(owner) = owner {
        sql.push_str(" WHERE owner = ?");
        vec![owner]
    } else {
        vec![]
    };

    let mut stmt = tx.prepare(&sql)?;
    let count: i64 = stmt.query_row(params_from_iter(params), |row| row.get(0))?;
    Ok(count as usize)
}

pub fn get_oldest_image_ids(tx: &Transaction, limit: usize) -> Result<Vec<u64>> {
    let mut stmt = tx.prepare("SELECT id FROM image ORDER BY id ASC LIMIT ?")?;
    let ids = stmt
        .query_map([limit], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<u64>>();
    Ok(ids)
}

pub fn delete_image(tx: &Transaction, id: u64) -> Result<()> {
    tx.execute("DELETE FROM image WHERE id = ?", params![id])?;
    Ok(())
}

pub fn delete_images_before_time(tx: &Transaction, time: u64) -> Result<usize> {
    let count = tx.execute("DELETE FROM image WHERE create_time < ?", params![time])?;
    Ok(count)
}

pub fn delete_images_batch(tx: &Transaction, ids: &[u64]) -> Result<usize> {
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("DELETE FROM image WHERE id IN ({})", placeholders);
    let count = tx.execute(&sql, params_from_iter(ids.iter()))?;
    Ok(count)
}
