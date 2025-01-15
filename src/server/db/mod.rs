mod sqlite;

pub mod cache;
pub mod config;
pub mod factory;

use std::cell::RefCell;
use std::sync::Mutex;

use anyhow::{bail, Result};
use cache::{Cache, DisableCache, UnionCache};
use sqlite::{Sqlite, SqliteTransaction};

use crate::types::request::Query;
use crate::types::user::RoleRule;

pub trait Connection<'a, T>
where
    T: Transaction + 'a,
{
    fn transaction(&'a mut self) -> Result<T>;
}

pub trait Transaction {
    fn create_user(&self, user: &UserRecord) -> Result<()>;
    fn get_user(&self, name: &str) -> Result<UserRecord>;
    fn list_users(&self) -> Result<Vec<UserRecord>>;
    fn is_user_exists(&self, name: &str) -> Result<bool>;
    fn update_user_password(&self, name: &str, hash: &str, salt: &str) -> Result<()>;
    fn update_user_time(&self, name: &str) -> Result<()>;
    fn delete_user(&self, name: &str) -> Result<()>;

    fn create_user_role(&self, name: &str, role: &str) -> Result<()>;
    fn delete_user_roles(&self, name: &str) -> Result<()>;
    fn is_role_in_use(&self, role: &str) -> Result<bool>;
    fn list_user_roles(&self, name: &str) -> Result<Vec<RoleRecord>>;

    fn create_role(&self, role: &RoleRecord) -> Result<()>;
    fn get_role(&self, name: &str) -> Result<RoleRecord>;
    fn list_roles(&self) -> Result<Vec<RoleRecord>>;
    fn is_role_exists(&self, name: &str) -> Result<bool>;
    fn update_role_rules(&self, name: &str, rules: &[RoleRule]) -> Result<()>;
    fn delete_role(&self, name: &str) -> Result<()>;

    fn create_text(&self, text: TextRecord) -> Result<TextRecord>;
    fn is_text_exists(&self, id: u64, owner: Option<&str>) -> Result<bool>;
    fn get_text(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<TextRecord>;
    fn get_latest_text(&self, owner: Option<&str>, simple: bool) -> Result<TextRecord>;
    fn list_texts(&self, query: Query, simple: bool) -> Result<Vec<TextRecord>>;
    fn count_texts(&self, owner: Option<&str>) -> Result<usize>;
    fn get_oldest_text_ids(&self, limit: usize) -> Result<Vec<u64>>;
    fn delete_text(&self, id: u64) -> Result<()>;
    fn delete_texts_before_time(&self, time: u64) -> Result<usize>;
    fn delete_texts_batch(&self, ids: &[u64]) -> Result<usize>;

    fn create_image(&self, image: ImageRecord) -> Result<ImageRecord>;
    fn is_image_exists(&self, id: u64, owner: Option<&str>) -> Result<bool>;
    fn get_image(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<ImageRecord>;
    fn get_latest_image(&self, owner: Option<&str>, simple: bool) -> Result<ImageRecord>;
    fn list_images(&self, query: Query) -> Result<Vec<ImageRecord>>;
    fn count_images(&self, owner: Option<&str>) -> Result<usize>;
    fn get_oldest_image_ids(&self, limit: usize) -> Result<Vec<u64>>;
    fn delete_image(&self, id: u64) -> Result<()>;
    fn delete_images_before_time(&self, time: u64) -> Result<usize>;
    fn delete_images_batch(&self, ids: &[u64]) -> Result<usize>;

    fn create_file(&self, file: FileRecord) -> Result<FileRecord>;
    fn is_file_exists(&self, id: u64, owner: Option<&str>) -> Result<bool>;
    fn get_file(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<FileRecord>;
    fn get_latest_file(&self, owner: Option<&str>, simple: bool) -> Result<FileRecord>;
    fn list_files(&self, query: Query) -> Result<Vec<FileRecord>>;
    fn count_files(&self, owner: Option<&str>) -> Result<usize>;
    fn get_oldest_file_ids(&self, limit: usize) -> Result<Vec<u64>>;
    fn delete_file(&self, id: u64) -> Result<()>;
    fn delete_files_before_time(&self, time: u64) -> Result<usize>;
    fn delete_files_batch(&self, ids: &[u64]) -> Result<usize>;

    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub name: String,
    pub hash: String,
    pub salt: String,

    pub create_time: u64,
    pub update_time: u64,
}

#[derive(Debug, Clone)]
pub struct RoleRecord {
    pub name: String,
    pub rules: Vec<RoleRule>,

    pub create_time: u64,
    pub update_time: u64,
}

#[derive(Debug, Clone)]
pub struct TextRecord {
    pub id: u64,
    pub content: String,
    pub hash: String,

    pub size: u64,

    pub owner: String,
    pub create_time: u64,
}

#[derive(Debug, Clone)]
pub struct ImageRecord {
    pub id: u64,
    pub data: Vec<u8>,
    pub hash: String,

    pub size: u64,

    pub owner: String,
    pub create_time: u64,
}

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub id: u64,
    pub name: String,

    pub data: Vec<u8>,
    pub hash: String,

    pub size: u64,

    pub mode: u32,

    pub owner: String,
    pub create_time: u64,
}

pub struct Database {
    ctx: Mutex<DatabaseContext>,
}

pub enum UnionConnection {
    Sqlite(Sqlite),
}

enum UnionTransaction<'a> {
    Sqlite(SqliteTransaction<'a>),
}

struct DatabaseContext {
    conn: RefCell<UnionConnection>,
    cache: Option<UnionCache>,
    no_cache: DisableCache,
}

impl Database {
    pub fn new(conn: UnionConnection, cache: Option<UnionCache>) -> Self {
        Self {
            ctx: Mutex::new(DatabaseContext {
                conn: RefCell::new(conn),
                cache,
                no_cache: DisableCache,
            }),
        }
    }

    pub fn with_transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&dyn Transaction, &dyn Cache) -> Result<T>,
    {
        let ctx = match self.ctx.lock() {
            Ok(ctx) => ctx,
            Err(e) => bail!("failed to lock database: {e:#}"),
        };
        let mut conn = ctx.conn.borrow_mut();
        let tx = conn.transaction()?;

        let result = if let Some(ref cache) = ctx.cache {
            f(&tx, cache)
        } else {
            f(&tx, &ctx.no_cache)
        };

        if result.is_ok() {
            tx.commit()
        } else {
            tx.rollback()
        }?;

        result
    }
}

impl<'a> Connection<'a, UnionTransaction<'a>> for UnionConnection {
    fn transaction(&'a mut self) -> Result<UnionTransaction<'a>> {
        match self {
            UnionConnection::Sqlite(sqlite) => sqlite.transaction().map(UnionTransaction::Sqlite),
        }
    }
}

impl Transaction for UnionTransaction<'_> {
    fn create_user(&self, user: &UserRecord) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_user(user),
        }
    }

    fn get_user(&self, name: &str) -> Result<UserRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_user(name),
        }
    }

    fn list_users(&self) -> Result<Vec<UserRecord>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.list_users(),
        }
    }

    fn is_user_exists(&self, name: &str) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.is_user_exists(name),
        }
    }

    fn update_user_password(&self, name: &str, hash: &str, salt: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.update_user_password(name, hash, salt),
        }
    }

    fn update_user_time(&self, name: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.update_user_time(name),
        }
    }

    fn delete_user(&self, name: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_user(name),
        }
    }

    fn create_user_role(&self, name: &str, role: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_user_role(name, role),
        }
    }

    fn delete_user_roles(&self, name: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_user_roles(name),
        }
    }

    fn is_role_in_use(&self, role: &str) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.is_role_in_use(role),
        }
    }

    fn list_user_roles(&self, name: &str) -> Result<Vec<RoleRecord>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.list_user_roles(name),
        }
    }

    fn create_role(&self, role: &RoleRecord) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_role(role),
        }
    }

    fn get_role(&self, name: &str) -> Result<RoleRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_role(name),
        }
    }

    fn list_roles(&self) -> Result<Vec<RoleRecord>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.list_roles(),
        }
    }

    fn is_role_exists(&self, name: &str) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.is_role_exists(name),
        }
    }

    fn update_role_rules(&self, name: &str, rules: &[RoleRule]) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.update_role_rules(name, rules),
        }
    }

    fn delete_role(&self, name: &str) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_role(name),
        }
    }

    fn create_text(&self, text: TextRecord) -> Result<TextRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_text(text),
        }
    }

    fn is_text_exists(&self, id: u64, owner: Option<&str>) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.is_text_exists(id, owner),
        }
    }

    fn get_text(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<TextRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_text(id, owner, simple),
        }
    }

    fn get_latest_text(&self, owner: Option<&str>, simple: bool) -> Result<TextRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_latest_text(owner, simple),
        }
    }

    fn list_texts(&self, query: Query, simple: bool) -> Result<Vec<TextRecord>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.list_texts(query, simple),
        }
    }

    fn count_texts(&self, owner: Option<&str>) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.count_texts(owner),
        }
    }

    fn get_oldest_text_ids(&self, limit: usize) -> Result<Vec<u64>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_oldest_text_ids(limit),
        }
    }

    fn delete_text(&self, id: u64) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_text(id),
        }
    }

    fn delete_texts_before_time(&self, time: u64) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_texts_before_time(time),
        }
    }

    fn delete_texts_batch(&self, ids: &[u64]) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_texts_batch(ids),
        }
    }

    fn create_image(&self, image: ImageRecord) -> Result<ImageRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_image(image),
        }
    }

    fn is_image_exists(&self, id: u64, owner: Option<&str>) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.is_image_exists(id, owner),
        }
    }

    fn get_image(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<ImageRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_image(id, owner, simple),
        }
    }

    fn get_latest_image(&self, owner: Option<&str>, simple: bool) -> Result<ImageRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_latest_image(owner, simple),
        }
    }

    fn list_images(&self, query: Query) -> Result<Vec<ImageRecord>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.list_images(query),
        }
    }

    fn count_images(&self, owner: Option<&str>) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.count_images(owner),
        }
    }

    fn get_oldest_image_ids(&self, limit: usize) -> Result<Vec<u64>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_oldest_image_ids(limit),
        }
    }

    fn delete_image(&self, id: u64) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_image(id),
        }
    }

    fn delete_images_before_time(&self, time: u64) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_images_before_time(time),
        }
    }

    fn delete_images_batch(&self, ids: &[u64]) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_images_batch(ids),
        }
    }

    fn create_file(&self, file: FileRecord) -> Result<FileRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.create_file(file),
        }
    }

    fn is_file_exists(&self, id: u64, owner: Option<&str>) -> Result<bool> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.is_file_exists(id, owner),
        }
    }

    fn get_file(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<FileRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_file(id, owner, simple),
        }
    }

    fn get_latest_file(&self, owner: Option<&str>, simple: bool) -> Result<FileRecord> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_latest_file(owner, simple),
        }
    }

    fn list_files(&self, query: Query) -> Result<Vec<FileRecord>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.list_files(query),
        }
    }

    fn count_files(&self, owner: Option<&str>) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.count_files(owner),
        }
    }

    fn get_oldest_file_ids(&self, limit: usize) -> Result<Vec<u64>> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.get_oldest_file_ids(limit),
        }
    }

    fn delete_file(&self, id: u64) -> Result<()> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_file(id),
        }
    }

    fn delete_files_before_time(&self, time: u64) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_files_before_time(time),
        }
    }

    fn delete_files_batch(&self, ids: &[u64]) -> Result<usize> {
        match self {
            UnionTransaction::Sqlite(tx) => tx.delete_files_batch(ids),
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
