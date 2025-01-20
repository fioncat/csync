mod sqlite;

#[cfg(test)]
mod tests;

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

/// Database connection trait that can create transactions
pub trait Connection<'a, T>
where
    T: Transaction + 'a,
{
    /// Creates a new transaction from the connection
    fn transaction(&'a mut self) -> Result<T>;
}

/// Database transaction trait that defines all database operations
pub trait Transaction {
    // User operations
    /// Creates a new user record
    fn create_user(&self, user: &UserRecord) -> Result<()>;
    /// Retrieves a user by name
    fn get_user(&self, name: &str) -> Result<UserRecord>;
    /// Lists all users
    fn list_users(&self) -> Result<Vec<UserRecord>>;
    /// Checks if a user exists
    fn is_user_exists(&self, name: &str) -> Result<bool>;
    /// Updates user's password hash and salt
    fn update_user_password(&self, name: &str, hash: &str, salt: &str) -> Result<()>;
    /// Updates user's last update time
    fn update_user_time(&self, name: &str) -> Result<()>;
    /// Deletes a user by name
    fn delete_user(&self, name: &str) -> Result<()>;

    // User-Role operations
    /// Assigns a role to a user
    fn create_user_role(&self, name: &str, role: &str) -> Result<()>;
    /// Removes all roles from a user
    fn delete_user_roles(&self, name: &str) -> Result<()>;
    /// Checks if a role is assigned to any user
    fn is_role_in_use(&self, role: &str) -> Result<bool>;
    /// Lists all roles assigned to a user
    fn list_user_roles(&self, name: &str) -> Result<Vec<RoleRecord>>;

    // Role operations
    /// Creates a new role
    fn create_role(&self, role: &RoleRecord) -> Result<()>;
    /// Retrieves a role by name
    fn get_role(&self, name: &str) -> Result<RoleRecord>;
    /// Lists all roles
    fn list_roles(&self) -> Result<Vec<RoleRecord>>;
    /// Checks if a role exists
    fn is_role_exists(&self, name: &str) -> Result<bool>;
    /// Updates role's rules
    fn update_role_rules(&self, name: &str, rules: &[RoleRule]) -> Result<()>;
    /// Deletes a role by name
    fn delete_role(&self, name: &str) -> Result<()>;

    // Text operations
    /// Creates a new text record
    fn create_text(&self, text: TextRecord) -> Result<TextRecord>;
    /// Checks if a text exists
    fn is_text_exists(&self, id: u64, owner: Option<&str>) -> Result<bool>;
    /// Retrieves a text by ID
    fn get_text(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<TextRecord>;
    /// Gets the latest text for an owner
    fn get_latest_text(&self, owner: Option<&str>, simple: bool) -> Result<TextRecord>;
    /// Lists texts based on query
    fn list_texts(&self, query: Query, simple: bool) -> Result<Vec<TextRecord>>;
    /// Counts texts for an owner
    fn count_texts(&self, owner: Option<&str>) -> Result<usize>;
    /// Gets IDs of oldest texts
    fn get_oldest_text_ids(&self, limit: usize) -> Result<Vec<u64>>;
    /// Deletes a text by ID
    fn delete_text(&self, id: u64) -> Result<()>;
    /// Deletes texts older than specified time
    fn delete_texts_before_time(&self, time: u64) -> Result<usize>;
    /// Deletes multiple texts by IDs
    fn delete_texts_batch(&self, ids: &[u64]) -> Result<usize>;

    // Image operations
    /// Creates a new image record
    fn create_image(&self, image: ImageRecord) -> Result<ImageRecord>;
    /// Checks if an image exists
    fn is_image_exists(&self, id: u64, owner: Option<&str>) -> Result<bool>;
    /// Retrieves an image by ID
    fn get_image(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<ImageRecord>;
    /// Gets the latest image for an owner
    fn get_latest_image(&self, owner: Option<&str>, simple: bool) -> Result<ImageRecord>;
    /// Lists images based on query
    fn list_images(&self, query: Query) -> Result<Vec<ImageRecord>>;
    /// Counts images for an owner
    fn count_images(&self, owner: Option<&str>) -> Result<usize>;
    /// Gets IDs of oldest images
    fn get_oldest_image_ids(&self, limit: usize) -> Result<Vec<u64>>;
    /// Deletes an image by ID
    fn delete_image(&self, id: u64) -> Result<()>;
    /// Deletes images older than specified time
    fn delete_images_before_time(&self, time: u64) -> Result<usize>;
    /// Deletes multiple images by IDs
    fn delete_images_batch(&self, ids: &[u64]) -> Result<usize>;

    // File operations
    /// Creates a new file record
    fn create_file(&self, file: FileRecord) -> Result<FileRecord>;
    /// Checks if a file exists
    fn is_file_exists(&self, id: u64, owner: Option<&str>) -> Result<bool>;
    /// Retrieves a file by ID
    fn get_file(&self, id: u64, owner: Option<&str>, simple: bool) -> Result<FileRecord>;
    /// Gets the latest file for an owner
    fn get_latest_file(&self, owner: Option<&str>, simple: bool) -> Result<FileRecord>;
    /// Lists files based on query
    fn list_files(&self, query: Query) -> Result<Vec<FileRecord>>;
    /// Counts files for an owner
    fn count_files(&self, owner: Option<&str>) -> Result<usize>;
    /// Gets IDs of oldest files
    fn get_oldest_file_ids(&self, limit: usize) -> Result<Vec<u64>>;
    /// Deletes a file by ID
    fn delete_file(&self, id: u64) -> Result<()>;
    /// Deletes files older than specified time
    fn delete_files_before_time(&self, time: u64) -> Result<usize>;
    /// Deletes multiple files by IDs
    fn delete_files_batch(&self, ids: &[u64]) -> Result<usize>;

    /// Commits the transaction
    fn commit(self) -> Result<()>;
    /// Rolls back the transaction
    fn rollback(self) -> Result<()>;
}

/// Record structure for user information
#[derive(Debug, Clone, PartialEq)]
pub struct UserRecord {
    /// User's unique name
    pub name: String,
    /// Password hash
    pub hash: String,
    /// Salt used for password hashing
    pub salt: String,
    /// User creation timestamp
    pub create_time: u64,
    /// Last update timestamp
    pub update_time: u64,
}

/// Record structure for role information
#[derive(Debug, Clone, PartialEq)]
pub struct RoleRecord {
    /// Role's unique name
    pub name: String,
    /// List of rules assigned to this role
    pub rules: Vec<RoleRule>,
    /// Role creation timestamp
    pub create_time: u64,
    /// Last update timestamp
    pub update_time: u64,
}

/// Record structure for text content
#[derive(Debug, Clone, PartialEq)]
pub struct TextRecord {
    /// Unique text ID
    pub id: u64,
    /// Text content
    pub content: String,
    /// Content hash
    pub hash: String,
    /// Content size in bytes
    pub size: u64,
    /// Owner's name
    pub owner: String,
    /// Creation timestamp
    pub create_time: u64,
}

/// Record structure for image content
#[derive(Debug, Clone, PartialEq)]
pub struct ImageRecord {
    /// Unique image ID
    pub id: u64,
    /// Binary image data
    pub data: Vec<u8>,
    /// Content hash
    pub hash: String,
    /// File size in bytes
    pub size: u64,
    /// Owner's name
    pub owner: String,
    /// Creation timestamp
    pub create_time: u64,
}

/// Record structure for file content
#[derive(Debug, Clone, PartialEq)]
pub struct FileRecord {
    /// Unique file ID
    pub id: u64,
    /// File name
    pub name: String,
    /// File binary data
    pub data: Vec<u8>,
    /// Content hash
    pub hash: String,
    /// File size in bytes
    pub size: u64,
    /// File mode/permissions
    pub mode: u32,
    /// Owner's name
    pub owner: String,
    /// Creation timestamp
    pub create_time: u64,
}

/// Main database structure supporting multiple backend implementations
pub struct Database {
    ctx: Mutex<DatabaseContext>,
}

/// Enum representing different supported database connections
pub enum UnionConnection {
    /// SQLite database connection
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
    /// Creates a new database instance with optional caching
    pub fn new(conn: UnionConnection, cache: Option<UnionCache>) -> Self {
        Self {
            ctx: Mutex::new(DatabaseContext {
                conn: RefCell::new(conn),
                cache,
                no_cache: DisableCache,
            }),
        }
    }

    /// Executes a function within a transaction context with optional caching support.
    ///
    /// This method provides a safe way to execute database operations within a transaction:
    /// - If the function `f` succeeds, the transaction will be committed
    /// - If the function `f` fails (returns an error), the transaction will be rolled back
    /// - If the transaction operations (commit/rollback) fail, the error will be returned
    ///
    /// The function `f` receives two parameters:
    /// - A reference to the transaction for database operations
    /// - A reference to the cache interface (enabled or disabled based on configuration)
    ///
    /// # Cache Support
    /// - If caching is enabled during database creation, `f` will receive an active cache implementation
    /// - If caching is disabled, `f` will receive a no-op cache implementation
    /// - Cache can be used to store frequently accessed data to improve performance
    ///
    /// # Example
    /// ```
    /// db.with_transaction(|tx, cache| {
    ///     // Perform database operations using tx
    ///     let text = tx.get_latest_text(None)?;
    ///
    ///     // Optionally use cache for frequently accessed data
    ///     cache.save_latest_text("Alice", text.clone())?;
    ///
    ///     Ok(text)
    /// })?;
    /// ```
    ///
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
