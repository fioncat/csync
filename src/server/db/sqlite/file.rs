use anyhow::Result;
use chrono::Local;
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, Transaction};

use crate::server::db::FileRecord;
use crate::types::request::Query;

const CREATE_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS file (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    data BLOB NOT NULL,
    hash TEXT NOT NULL,
    size INTEGER NOT NULL,
    mode INTEGER NOT NULL,
    owner TEXT NOT NULL,
    create_time INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_file_owner ON file(owner);
CREATE INDEX IF NOT EXISTS idx_file_create_time ON file(create_time);
"#;

pub fn create_file_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(CREATE_TABLES)?;
    Ok(())
}

pub fn create_file(tx: &Transaction, mut file: FileRecord) -> Result<FileRecord> {
    let now = Local::now().timestamp() as u64;
    tx.execute(
        "INSERT INTO file (name, data, hash, size, mode, owner, create_time) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![file.name, file.data, file.hash, file.size, file.mode, file.owner, now],
    )?;
    let id = tx.last_insert_rowid() as u64;
    file.id = id;
    file.create_time = now;
    Ok(file)
}

pub fn is_file_exists(tx: &Transaction, id: u64, owner: Option<&str>) -> Result<bool> {
    let sql = if owner.is_some() {
        "SELECT COUNT(*) FROM file WHERE id = ? AND owner = ?"
    } else {
        "SELECT COUNT(*) FROM file WHERE id = ?"
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

pub fn get_file(
    tx: &Transaction,
    id: u64,
    owner: Option<&str>,
    simple: bool,
) -> Result<FileRecord> {
    let sql = if simple {
        "SELECT id, name, hash, size, mode, owner, create_time FROM file WHERE id = ?"
    } else {
        "SELECT id, name, data, hash, size, mode, owner, create_time FROM file WHERE id = ?"
    };
    let mut sql = String::from(sql);
    let params = if let Some(owner) = owner {
        sql.push_str(" AND owner = ?");
        vec![Value::Integer(id as i64), Value::Text(String::from(owner))]
    } else {
        vec![Value::Integer(id as i64)]
    };

    let mut stmt = tx.prepare(&sql)?;
    let file = stmt.query_row(params_from_iter(params), |row| {
        let record = if simple {
            FileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                data: Vec::new(),
                hash: row.get(2)?,
                size: row.get(3)?,
                mode: row.get(4)?,
                owner: row.get(5)?,
                create_time: row.get(6)?,
            }
        } else {
            FileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                data: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
                mode: row.get(5)?,
                owner: row.get(6)?,
                create_time: row.get(7)?,
            }
        };
        Ok(record)
    })?;
    Ok(file)
}

pub fn get_latest_file(tx: &Transaction, owner: Option<&str>, simple: bool) -> Result<FileRecord> {
    let sql = if simple {
        "SELECT id, name, hash, size, mode, owner, create_time FROM file"
    } else {
        "SELECT id, name, data, hash, size, mode, owner, create_time FROM file"
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
    let file = stmt.query_row(params_from_iter(params), |row| {
        let record = if simple {
            FileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                data: Vec::new(),
                hash: row.get(2)?,
                size: row.get(3)?,
                mode: row.get(4)?,
                owner: row.get(5)?,
                create_time: row.get(6)?,
            }
        } else {
            FileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                data: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
                mode: row.get(5)?,
                owner: row.get(6)?,
                create_time: row.get(7)?,
            }
        };
        Ok(record)
    })?;
    Ok(file)
}

pub fn list_files(tx: &Transaction, query: Query) -> Result<Vec<FileRecord>> {
    let where_clause = query.generate_where("owner", "create_time");
    let limit_clause = query.generate_limit();
    let params = query.params();

    let sql = format!("SELECT id, name, hash, size, mode, owner, create_time FROM file {where_clause} ORDER BY create_time DESC {limit_clause}");

    let mut stmt = tx.prepare(&sql)?;
    let files = stmt
        .query_map(params_from_iter(params), |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                data: Vec::new(),
                hash: row.get(2)?,
                size: row.get(3)?,
                mode: row.get(4)?,
                owner: row.get(5)?,
                create_time: row.get(6)?,
            })
        })?
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    Ok(files)
}

pub fn count_files(tx: &Transaction, owner: Option<&str>) -> Result<usize> {
    let mut sql = String::from("SELECT COUNT(*) FROM file");
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

pub fn get_oldest_file_ids(tx: &Transaction, limit: usize) -> Result<Vec<u64>> {
    let mut stmt = tx.prepare("SELECT id FROM file ORDER BY id ASC LIMIT ?")?;
    let names = stmt
        .query_map([limit], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<u64>>();
    Ok(names)
}

pub fn delete_file(tx: &Transaction, id: u64) -> Result<()> {
    tx.execute("DELETE FROM file WHERE id = ?", [id])?;
    Ok(())
}

pub fn delete_files_before_time(tx: &Transaction, time: u64) -> Result<usize> {
    let count = tx.execute("DELETE FROM file WHERE create_time < ?", [time])?;
    Ok(count)
}

pub fn delete_files_batch(tx: &Transaction, ids: &[u64]) -> Result<usize> {
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("DELETE FROM file WHERE name IN ({})", placeholders);
    let count = tx.execute(&sql, params_from_iter(ids.iter()))?;
    Ok(count)
}
