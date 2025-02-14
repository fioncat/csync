mod file;
mod image;
mod text;
mod user;

pub mod config;
pub mod factory;

use std::path::Path;

use anyhow::Result;
use csync_misc::types::request::Query;
use csync_misc::types::user::RoleRule;
use rusqlite::types::Value;
use rusqlite::Connection as RawConnection;
use rusqlite::Transaction as RawTransaction;

use crate::db::TextRecord;

use super::FileRecord;
use super::ImageRecord;
use super::{Connection, RoleRecord, Transaction, UserRecord};

/// SQLite-based database implementation. This is the simplest database type,
/// perfect for single-node deployments. Supports both file-based and in-memory
/// database types.
pub struct Sqlite {
    conn: RawConnection,
}

/// SQLite transaction for executing database operations
pub struct SqliteTransaction<'a> {
    tx: RawTransaction<'a>,
}

impl Sqlite {
    /// Opens a SQLite database file. Creates one if it doesn't exist.
    /// Also initializes all required database tables.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = RawConnection::open(path)?;
        Self::init_tables(&conn)?;
        Ok(Self { conn })
    }

    /// Creates a new in-memory database. Database content will be lost when the program exits.
    /// Also initializes all required database tables.
    /// This method is recommended for testing only.
    pub fn memory() -> Result<Self> {
        let conn = RawConnection::open_in_memory()?;
        Self::init_tables(&conn)?;
        Ok(Self { conn })
    }

    fn init_tables(db: &RawConnection) -> Result<()> {
        user::create_user_tables(db)?;
        text::create_text_tables(db)?;
        image::create_image_tables(db)?;
        file::create_file_tables(db)?;
        Ok(())
    }
}

impl<'a> Connection<'a, SqliteTransaction<'a>> for Sqlite {
    fn transaction(&'a mut self) -> Result<SqliteTransaction<'a>> {
        let tx = self.conn.transaction()?;
        Ok(SqliteTransaction { tx })
    }
}

impl Transaction for SqliteTransaction<'_> {
    fn create_user(&self, user: &UserRecord) -> Result<()> {
        user::create_user(&self.tx, user)
    }

    fn get_user(&self, name: &str) -> Result<UserRecord> {
        user::get_user(&self.tx, name)
    }

    fn list_users(&self) -> Result<Vec<UserRecord>> {
        user::list_users(&self.tx)
    }

    fn is_user_exists(&self, name: &str) -> Result<bool> {
        user::is_user_exists(&self.tx, name)
    }

    fn update_user_password(&self, name: &str, hash: &str, salt: &str) -> Result<()> {
        user::update_user_password(&self.tx, name, hash, salt)
    }

    fn update_user_time(&self, name: &str) -> Result<()> {
        user::update_user_time(&self.tx, name)
    }

    fn delete_user(&self, name: &str) -> Result<()> {
        user::delete_user(&self.tx, name)
    }

    fn create_user_role(&self, name: &str, role: &str) -> Result<()> {
        user::create_user_role(&self.tx, name, role)
    }

    fn delete_user_roles(&self, name: &str) -> Result<()> {
        user::delete_user_roles(&self.tx, name)
    }

    fn is_role_in_use(&self, role: &str) -> Result<bool> {
        user::is_role_in_use(&self.tx, role)
    }

    fn list_user_roles(&self, name: &str) -> Result<Vec<RoleRecord>> {
        user::list_user_roles(&self.tx, name)
    }

    fn create_role(&self, role: &RoleRecord) -> Result<()> {
        user::create_role(&self.tx, role)
    }

    fn get_role(&self, name: &str) -> Result<RoleRecord> {
        user::get_role(&self.tx, name)
    }

    fn list_roles(&self) -> Result<Vec<RoleRecord>> {
        user::list_roles(&self.tx)
    }

    fn is_role_exists(&self, name: &str) -> Result<bool> {
        user::is_role_exists(&self.tx, name)
    }

    fn update_role_rules(&self, name: &str, rules: &[RoleRule]) -> Result<()> {
        user::update_role_rules(&self.tx, name, rules)
    }

    fn delete_role(&self, name: &str) -> Result<()> {
        user::delete_role(&self.tx, name)
    }

    fn create_text(&self, text: TextRecord) -> Result<TextRecord> {
        text::create_text(&self.tx, text)
    }

    fn update_text_pin(&self, id: u64, pin: bool) -> Result<()> {
        text::update_text_pin(&self.tx, id, pin)
    }

    fn is_text_exists(&self, id: u64, owner: Option<&str>) -> Result<bool> {
        text::is_text_exists(&self.tx, id, owner)
    }

    fn get_text(&self, id: u64, owner: Option<&str>, head: bool) -> Result<TextRecord> {
        text::get_text(&self.tx, id, owner, head)
    }

    fn get_latest_text(&self, owner: Option<&str>, head: bool) -> Result<TextRecord> {
        text::get_latest_text(&self.tx, owner, head)
    }

    fn list_texts(&self, query: Query, head: bool) -> Result<Vec<TextRecord>> {
        text::list_texts(&self.tx, query, head)
    }

    fn count_texts(&self, owner: Option<&str>, with_pin: bool) -> Result<usize> {
        text::count_texts(&self.tx, owner, with_pin)
    }

    fn get_oldest_text_ids(&self, limit: usize) -> Result<Vec<u64>> {
        text::get_oldest_text_ids(&self.tx, limit)
    }

    fn delete_text(&self, id: u64) -> Result<()> {
        text::delete_text(&self.tx, id)
    }

    fn delete_texts_before_time(&self, time: u64) -> Result<usize> {
        text::delete_texts_before_time(&self.tx, time)
    }

    fn delete_texts_batch(&self, ids: &[u64]) -> Result<usize> {
        text::delete_texts_batch(&self.tx, ids)
    }

    fn create_image(&self, image: ImageRecord) -> Result<ImageRecord> {
        image::create_image(&self.tx, image)
    }

    fn update_image_pin(&self, id: u64, pin: bool) -> Result<()> {
        image::update_image_pin(&self.tx, id, pin)
    }

    fn is_image_exists(&self, id: u64, owner: Option<&str>) -> Result<bool> {
        image::is_image_exists(&self.tx, id, owner)
    }

    fn get_image(&self, id: u64, owner: Option<&str>, head: bool) -> Result<ImageRecord> {
        image::get_image(&self.tx, id, owner, head)
    }

    fn get_latest_image(&self, owner: Option<&str>, head: bool) -> Result<ImageRecord> {
        image::get_latest_image(&self.tx, owner, head)
    }

    fn list_images(&self, query: Query) -> Result<Vec<ImageRecord>> {
        image::list_images(&self.tx, query)
    }

    fn count_images(&self, owner: Option<&str>, with_pin: bool) -> Result<usize> {
        image::count_images(&self.tx, owner, with_pin)
    }

    fn get_oldest_image_ids(&self, limit: usize) -> Result<Vec<u64>> {
        image::get_oldest_image_ids(&self.tx, limit)
    }

    fn delete_image(&self, id: u64) -> Result<()> {
        image::delete_image(&self.tx, id)
    }

    fn delete_images_before_time(&self, time: u64) -> Result<usize> {
        image::delete_images_before_time(&self.tx, time)
    }

    fn delete_images_batch(&self, ids: &[u64]) -> Result<usize> {
        image::delete_images_batch(&self.tx, ids)
    }

    fn create_file(&self, file: FileRecord) -> Result<FileRecord> {
        file::create_file(&self.tx, file)
    }

    fn update_file_pin(&self, id: u64, pin: bool) -> Result<()> {
        file::update_file_pin(&self.tx, id, pin)
    }

    fn is_file_exists(&self, id: u64, owner: Option<&str>) -> Result<bool> {
        file::is_file_exists(&self.tx, id, owner)
    }

    fn get_file(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<FileRecord> {
        file::get_file(&self.tx, id, owner, simple)
    }

    fn get_latest_file(&self, owner: Option<&str>, simple: bool) -> Result<FileRecord> {
        file::get_latest_file(&self.tx, owner, simple)
    }

    fn list_files(&self, query: Query) -> Result<Vec<FileRecord>> {
        file::list_files(&self.tx, query)
    }

    fn count_files(&self, owner: Option<&str>, with_pin: bool) -> Result<usize> {
        file::count_files(&self.tx, owner, with_pin)
    }

    fn get_oldest_file_ids(&self, limit: usize) -> Result<Vec<u64>> {
        file::get_oldest_file_ids(&self.tx, limit)
    }

    fn delete_file(&self, id: u64) -> Result<()> {
        file::delete_file(&self.tx, id)
    }

    fn delete_files_before_time(&self, time: u64) -> Result<usize> {
        file::delete_files_before_time(&self.tx, time)
    }

    fn delete_files_batch(&self, ids: &[u64]) -> Result<usize> {
        file::delete_files_batch(&self.tx, ids)
    }

    fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }

    fn rollback(self) -> Result<()> {
        self.tx.rollback()?;
        Ok(())
    }
}

/// Converts query conditions into SQL parameters
pub fn convert_query_to_params(query: Query) -> Vec<Value> {
    let mut params = vec![];
    if let Some(search) = query.search {
        params.push(Value::Text(format!("%{}%", search)));
    }
    if let Some(since) = query.since {
        params.push(Value::Integer(since as i64));
    }
    if let Some(until) = query.until {
        params.push(Value::Integer(until as i64));
    }
    if let Some(owner) = query.owner {
        params.push(Value::Text(owner));
    }
    if let Some(hash) = query.hash {
        params.push(Value::Text(hash));
    }
    if let Some(limit) = query.limit {
        params.push(Value::Integer(limit as i64));
    }
    if let Some(offset) = query.offset {
        params.push(Value::Integer(offset as i64));
    }
    params
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::db::tests::run_all_db_tests;
    use crate::db::{Database, UnionConnection};

    use super::*;

    #[test]
    fn test_memory() {
        let sqlite = Sqlite::memory().unwrap();
        let conn = UnionConnection::Sqlite(sqlite);
        let db = Database::new(conn, None);

        run_all_db_tests(&db);
    }

    #[test]
    fn test_file() {
        let path = "/tmp/test_csync.db";
        let _ = fs::remove_file(path);

        let sqlite = Sqlite::open(Path::new(path)).unwrap();
        let conn = UnionConnection::Sqlite(sqlite);
        let db = Database::new(conn, None);

        run_all_db_tests(&db);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_query_to_params() {
        // Test params generation
        let query = Query {
            offset: Some(5),
            limit: Some(10),
            search: Some("test".to_string()),
            since: Some(1000),
            until: Some(2000),
            owner: Some("user".to_string()),
            hash: Some("hash123".to_string()),
        };

        let params = convert_query_to_params(query);
        assert_eq!(params.len(), 7);

        // Check search parameter
        if let Value::Text(search) = &params[0] {
            assert_eq!(search, "%test%");
        } else {
            panic!("Expected Text value for search");
        }

        // Check since parameter
        if let Value::Integer(since) = params[1] {
            assert_eq!(since, 1000);
        } else {
            panic!("Expected Integer value for since");
        }

        // Check until parameter
        if let Value::Integer(until) = params[2] {
            assert_eq!(until, 2000);
        } else {
            panic!("Expected Integer value for until");
        }

        // Check owner parameter
        if let Value::Text(owner) = &params[3] {
            assert_eq!(owner, "user");
        } else {
            panic!("Expected Text value for owner");
        }

        // Check hash parameter
        if let Value::Text(hash) = &params[4] {
            assert_eq!(hash, "hash123");
        } else {
            panic!("Expected Text value for hash");
        }

        // Check limit parameter
        if let Value::Integer(limit) = params[5] {
            assert_eq!(limit, 10);
        } else {
            panic!("Expected Integer value for limit");
        }

        // Check offset parameter
        if let Value::Integer(offset) = params[6] {
            assert_eq!(offset, 5);
        } else {
            panic!("Expected Integer value for offset");
        }
    }
}
