mod user;

use anyhow::Result;
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::types::user::{Role, User};

use super::Storage;

#[derive(Debug)]
pub struct SqliteStore {
    // TODO: Using a Mutex for the database connection is a temporary solution for simplicity.
    // This approach may cause performance bottlenecks under high concurrency.
    // A connection pool should be implemented for better scalability in production.
    conn: Mutex<Connection>,
}

#[async_trait::async_trait]
impl Storage for SqliteStore {
    async fn put_user(&self, user: &User) -> Result<()> {
        todo!()
    }

    async fn get_user(&self, name: &str) -> Result<Option<User>> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        todo!()
    }

    async fn delete_user(&self, name: &str) -> Result<()> {
        todo!()
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        todo!()
    }

    async fn validate_user(&self, name: &str, password: &str) -> Result<bool> {
        todo!()
    }

    async fn put_role(&self, role: &Role) -> Result<()> {
        todo!()
    }

    async fn get_role(&self, name: &str) -> Result<Option<Role>> {
        todo!()
    }

    async fn delete_role(&self, name: &str) -> Result<()> {
        todo!()
    }

    async fn list_roles(&self) -> Result<Vec<Role>> {
        todo!()
    }
}
