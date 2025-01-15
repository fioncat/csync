use anyhow::Result;
use chrono::Local;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Transaction};

use crate::server::db::TextRecord;
use crate::types::request::Query;

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS text (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    content TEXT NOT NULL,
    hash TEXT NOT NULL,
    size INTEGER NOT NULL,
    owner TEXT NOT NULL,
    create_time INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_text_owner ON text(owner);
CREATE INDEX IF NOT EXISTS idx_text_hash ON text(hash);
CREATE INDEX IF NOT EXISTS idx_text_create_time ON text(create_time);
"#;

pub fn create_text_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}

pub fn create_text(tx: &Transaction, mut text: TextRecord) -> Result<TextRecord> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO text (content, hash, size, owner, create_time) VALUES (?, ?, ?, ?, ?)",
        params![text.content, text.hash, text.size, text.owner, now],
    )?;
    let id = tx.last_insert_rowid() as u64;
    text.id = id;
    text.create_time = now;
    Ok(text)
}

pub fn is_text_exists(tx: &Transaction, id: u64, owner: Option<&str>) -> Result<bool> {
    let sql = if owner.is_some() {
        "SELECT COUNT(*) FROM text WHERE id = ? AND owner = ?"
    } else {
        "SELECT COUNT(*) FROM text WHERE id = ?"
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

pub fn get_text(
    tx: &Transaction,
    id: u64,
    owner: Option<&str>,
    simple: bool,
) -> Result<TextRecord> {
    let sql = if simple {
        "SELECT id, hash, size, owner, create_time FROM text WHERE id = ?"
    } else {
        "SELECT id, content, hash, size, owner, create_time FROM text WHERE id = ?"
    };

    let mut sql = String::from(sql);

    let params = if let Some(owner) = owner {
        sql.push_str(" AND owner = ?");
        vec![Value::Integer(id as i64), Value::Text(String::from(owner))]
    } else {
        vec![Value::Integer(id as i64)]
    };

    let mut stmt = tx.prepare(&sql)?;
    let text = stmt.query_row(params_from_iter(params), |row| {
        let record = if simple {
            TextRecord {
                id: row.get(0)?,
                content: String::new(),
                hash: row.get(1)?,
                size: row.get(2)?,
                owner: row.get(3)?,
                create_time: row.get(4)?,
            }
        } else {
            TextRecord {
                id: row.get(0)?,
                content: row.get(1)?,
                hash: row.get(2)?,
                size: row.get(3)?,
                owner: row.get(4)?,
                create_time: row.get(5)?,
            }
        };
        Ok(record)
    })?;
    Ok(text)
}

pub fn get_latest_text(tx: &Transaction, owner: Option<&str>, simple: bool) -> Result<TextRecord> {
    let sql = if simple {
        "SELECT id, hash, size, owner, create_time FROM text"
    } else {
        "SELECT id, content, hash, size, owner, create_time FROM text"
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
    let text = stmt.query_row(params_from_iter(params), |row| {
        let record = if simple {
            TextRecord {
                id: row.get(0)?,
                content: String::new(),
                hash: row.get(1)?,
                size: row.get(2)?,
                owner: row.get(3)?,
                create_time: row.get(4)?,
            }
        } else {
            TextRecord {
                id: row.get(0)?,
                content: row.get(1)?,
                hash: row.get(2)?,
                size: row.get(3)?,
                owner: row.get(4)?,
                create_time: row.get(5)?,
            }
        };
        Ok(record)
    })?;
    Ok(text)
}

pub fn list_texts(tx: &Transaction, query: Query, simple: bool) -> Result<Vec<TextRecord>> {
    let where_clause = query.generate_where("owner", "create_time");
    let limit_clause = query.generate_limit();
    let params = query.params();

    let sql = if simple {
        format!("SELECT id, hash, size, owner, create_time FROM text {where_clause} ORDER BY id DESC {limit_clause}")
    } else {
        format!("SELECT id, content, hash, size, owner, create_time FROM text {where_clause} ORDER BY id DESC {limit_clause}")
    };

    let mut stmt = tx.prepare(&sql)?;
    let texts = stmt
        .query_map(params_from_iter(params.iter()), |row| {
            let record = if simple {
                TextRecord {
                    id: row.get(0)?,
                    content: String::new(),
                    hash: row.get(1)?,
                    size: row.get(2)?,
                    owner: row.get(3)?,
                    create_time: row.get(4)?,
                }
            } else {
                TextRecord {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    hash: row.get(2)?,
                    size: row.get(3)?,
                    owner: row.get(4)?,
                    create_time: row.get(5)?,
                }
            };
            Ok(record)
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    Ok(texts)
}

pub fn count_texts(tx: &Transaction, owner: Option<&str>) -> Result<usize> {
    let mut sql = String::from("SELECT COUNT(*) FROM text");
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

pub fn get_oldest_text_ids(tx: &Transaction, limit: usize) -> Result<Vec<u64>> {
    let mut stmt = tx.prepare("SELECT id FROM text ORDER BY id ASC LIMIT ?")?;
    let ids = stmt
        .query_map([limit], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<u64>>();
    Ok(ids)
}

pub fn delete_text(tx: &Transaction, id: u64) -> Result<()> {
    tx.execute("DELETE FROM text WHERE id = ?", params![id])?;
    Ok(())
}

pub fn delete_texts_before_time(tx: &Transaction, time: u64) -> Result<usize> {
    let count = tx.execute("DELETE FROM text WHERE create_time < ?", params![time])?;
    Ok(count)
}

pub fn delete_texts_batch(tx: &Transaction, ids: &[u64]) -> Result<usize> {
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("DELETE FROM text WHERE id IN ({})", placeholders);
    let count = tx.execute(&sql, params_from_iter(ids.iter()))?;
    Ok(count)
}
