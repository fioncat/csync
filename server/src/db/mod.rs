mod sql;
mod sqlite;

#[cfg(test)]
mod tests;

pub mod config;
pub mod types;

use std::cell::RefCell;
use std::sync::Mutex;

use anyhow::{bail, Result};
use csync_misc::api::blob::Blob;
use csync_misc::api::metadata::{GetMetadataRequest, Metadata};
use csync_misc::api::user::{GetUserRequest, PatchUserRequest, User};
use sqlite::{SqliteConnection, SqliteTransaction};
use types::{Connection, CreateUserParams, PatchBlobParams, Transaction, UserPassword};

pub struct Database {
    conn: Mutex<RefCell<UnionConnection>>,
}

impl Database {
    pub fn new(conn: UnionConnection) -> Self {
        Self {
            conn: Mutex::new(RefCell::new(conn)),
        }
    }

    #[cfg(test)]
    pub fn new_test() -> Self {
        let conn = SqliteConnection::memory().unwrap();
        Self::new(UnionConnection::Sqlite(conn))
    }

    pub fn with_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn Transaction) -> Result<T>,
    {
        let conn = match self.conn.lock() {
            Ok(conn) => conn,
            Err(e) => bail!("failed to lock connection: {:#}", e),
        };
        let mut conn = conn.borrow_mut();
        let tx = conn.transaction()?;

        let result = f(&tx);

        if result.is_ok() {
            tx.commit()
        } else {
            tx.rollback()
        }?;

        result
    }
}

pub enum UnionConnection {
    Sqlite(SqliteConnection),
}

pub enum UnionTransaction<'a> {
    Sqlite(SqliteTransaction<'a>),
}

impl<'a> Connection<'a, UnionTransaction<'a>> for UnionConnection {
    fn transaction(&'a mut self) -> Result<UnionTransaction<'a>> {
        match self {
            UnionConnection::Sqlite(conn) => conn.transaction().map(UnionTransaction::Sqlite),
        }
    }
}

impl Transaction for UnionTransaction<'_> {
    fn create_blob(&self, params: types::CreateBlobParams) -> Result<u64> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_blob(params),
        }
    }

    fn update_blob(&self, params: PatchBlobParams) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.update_blob(params),
        }
    }

    fn delete_blob(&self, id: u64) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_blob(id),
        }
    }

    fn delete_blobs(&self, ids: Vec<u64>) -> Result<u64> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_blobs(ids),
        }
    }

    fn get_blob(&self, id: u64) -> Result<Blob> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_blob(id),
        }
    }

    fn has_blob(&self, id: u64) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.has_blob(id),
        }
    }

    fn get_metadata(&self, id: u64) -> Result<Metadata> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_metadata(id),
        }
    }

    fn count_metadatas(&self, req: GetMetadataRequest) -> Result<u64> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.count_metadatas(req),
        }
    }

    fn get_metadatas(&self, req: GetMetadataRequest) -> Result<Vec<Metadata>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_metadatas(req),
        }
    }

    fn create_user(&self, params: CreateUserParams) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_user(params),
        }
    }

    fn update_user(&self, patch: PatchUserRequest, update_time: u64) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.update_user(patch, update_time),
        }
    }

    fn delete_user(&self, name: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_user(name),
        }
    }

    fn has_user(&self, name: String) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.has_user(name),
        }
    }

    fn get_user_password(&self, name: String) -> Result<UserPassword> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_user_password(name),
        }
    }

    fn count_users(&self, req: GetUserRequest) -> Result<u64> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.count_users(req),
        }
    }

    fn get_users(&self, req: GetUserRequest) -> Result<Vec<User>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_users(req),
        }
    }

    fn commit(self) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.commit(),
        }
    }

    fn rollback(self) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.rollback(),
        }
    }
}
