mod cache;
mod sqlite;

pub mod config;
pub mod factory;

use anyhow::Result;

use crate::types::user::{Role, User};

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn put_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, name: &str) -> Result<Option<User>>;
    async fn delete_user(&self, name: &str) -> Result<()>;
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn validate_user(&self, name: &str, password: &str) -> Result<bool>;

    async fn put_role(&self, role: &Role) -> Result<()>;
    async fn get_role(&self, name: &str) -> Result<Option<Role>>;
    async fn delete_role(&self, name: &str) -> Result<()>;
    async fn list_roles(&self) -> Result<Vec<Role>>;
}
