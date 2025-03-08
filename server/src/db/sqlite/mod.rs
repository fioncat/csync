mod blob;
mod user;

pub mod config;

use std::path::Path;

use anyhow::{Context, Result};
use csync_misc::api::blob::Blob;
use csync_misc::api::metadata::{GetMetadataRequest, Metadata};
use csync_misc::api::user::{GetUserRequest, PatchUserRequest, User};
use csync_misc::api::Value;
use rusqlite::types::Value as DbValue;
use rusqlite::Connection as DbConnection;
use rusqlite::Transaction as DbTransaction;

use super::types::{Connection, PatchBlobParams, Transaction, UserPassword};
use super::types::{CreateBlobParams, CreateUserParams};

pub struct SqliteConnection {
    conn: DbConnection,
}

pub struct SqliteTransaction<'a> {
    tx: DbTransaction<'a>,
}

impl SqliteConnection {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = DbConnection::open(path)?;
        let conn = SqliteConnection { conn };
        conn.init_tables()?;
        Ok(conn)
    }

    pub fn memory() -> Result<Self> {
        let conn = DbConnection::open_in_memory()?;
        let conn = SqliteConnection { conn };
        conn.init_tables()?;
        Ok(conn)
    }

    fn init_tables(&self) -> Result<()> {
        blob::create_table(&self.conn)?;
        user::create_table(&self.conn)?;
        Ok(())
    }
}

impl<'a> Connection<'a, SqliteTransaction<'a>> for SqliteConnection {
    fn transaction(&'a mut self) -> Result<SqliteTransaction<'a>> {
        let tx = self.conn.transaction()?;
        Ok(SqliteTransaction { tx })
    }
}

impl Transaction for SqliteTransaction<'_> {
    fn create_blob(&self, params: CreateBlobParams) -> Result<u64> {
        blob::create(&self.tx, params)
    }

    fn update_blob(&self, params: PatchBlobParams) -> Result<()> {
        blob::update(&self.tx, params)
    }

    fn delete_blob(&self, id: u64) -> Result<()> {
        blob::delete(&self.tx, id)
    }

    fn delete_blobs(&self, ids: Vec<u64>) -> Result<u64> {
        blob::delete_batch(&self.tx, ids)
    }

    fn get_blob(&self, id: u64) -> Result<Blob> {
        blob::get(&self.tx, id)
    }

    fn has_blob(&self, id: u64) -> Result<bool> {
        blob::has(&self.tx, id)
    }

    fn get_metadata(&self, id: u64) -> Result<Metadata> {
        blob::get_metadata(&self.tx, id)
    }

    fn count_metadatas(&self, req: GetMetadataRequest) -> Result<u64> {
        blob::count_metadatas(&self.tx, req)
    }

    fn get_metadatas(&self, req: GetMetadataRequest) -> Result<Vec<Metadata>> {
        blob::get_metadatas(&self.tx, req)
    }

    fn create_user(&self, params: CreateUserParams) -> Result<()> {
        user::create(&self.tx, params)
    }

    fn update_user(&self, patch: PatchUserRequest, update_time: u64) -> Result<()> {
        user::update(&self.tx, patch, update_time)
    }

    fn delete_user(&self, name: &str) -> Result<()> {
        user::delete(&self.tx, name)
    }

    fn has_user(&self, name: String) -> Result<bool> {
        user::has(&self.tx, name)
    }

    fn get_user_password(&self, name: String) -> Result<UserPassword> {
        user::get_user_password(&self.tx, name)
    }

    fn count_users(&self, req: GetUserRequest) -> Result<u64> {
        user::count_users(&self.tx, req)
    }

    fn get_users(&self, req: GetUserRequest) -> Result<Vec<User>> {
        user::get_users(&self.tx, req)
    }

    fn commit(self) -> Result<()> {
        self.tx
            .commit()
            .context("failed to commit sqlite transaction")
    }

    fn rollback(self) -> Result<()> {
        self.tx
            .rollback()
            .context("failed to rollback sqlite transaction")
    }
}

pub fn convert_values(values: Vec<Value>) -> Vec<DbValue> {
    values
        .into_iter()
        .map(|v| match v {
            Value::Text(text) => DbValue::Text(text),
            Value::Integer(int) => DbValue::Integer(int as i64),
            Value::Bool(b) => DbValue::Integer(b as i64),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::db::tests::run_tests;
    use crate::db::{Database, UnionConnection};

    use super::*;

    #[test]
    fn test_memory() {
        let conn = SqliteConnection::memory().unwrap();
        let db = Database::new(UnionConnection::Sqlite(conn));

        run_tests(&db);
    }

    #[test]
    fn test_file() {
        let path = PathBuf::from("testdata/sqlite.db");
        let _ = fs::remove_file(&path);

        let conn = SqliteConnection::open(&path).unwrap();
        let db = Database::new(UnionConnection::Sqlite(conn));

        run_tests(&db);
    }
}
