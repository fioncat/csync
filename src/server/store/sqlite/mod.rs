mod user;

use std::path::Path;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use tokio::sync::Mutex;

use crate::types::user::{Password, Role, User};

use super::Storage;

use user::*;

/// SQLite-based storage implementation.
///
/// This implementation uses SQLite as the backend database and provides
/// thread-safe access through a Mutex-protected connection.
#[derive(Debug)]
pub struct SqliteStore {
    // TODO: Using a Mutex for the database connection is a temporary solution for simplicity.
    // This approach may cause performance bottlenecks under high concurrency.
    // A connection pool should be implemented for better scalability in production.
    conn: Mutex<Connection>,
}

impl SqliteStore {
    /// Opens a SQLite database at the specified path.
    ///
    /// Creates necessary tables if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the SQLite database file
    ///
    /// # Returns
    ///
    /// Returns a new `SqliteStore` instance on success, or an error if:
    /// - Failed to open/create database file
    /// - Failed to create required tables
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        create_user_tables(&conn).context("create user tables")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Creates an in-memory SQLite database for testing.
    ///
    /// Creates necessary tables in the memory database.
    ///
    /// # Returns
    ///
    /// Returns a new `SqliteStore` instance with an in-memory database on success,
    /// or an error if failed to create required tables.
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        create_user_tables(&conn).context("create user tables")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[async_trait::async_trait]
impl Storage for SqliteStore {
    async fn put_user(&self, user: &User) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        for role_name in user.role_names.iter() {
            if !check_role_exists(&tx, role_name)? {
                bail!("role '{}' does not exist", role_name);
            }
        }

        if !check_user_exists(&tx, &user.name)? {
            let password = match user.password {
                Some(ref p) => p,
                None => bail!("password is required to create user"),
            };
            if user.role_names.is_empty() {
                bail!("user must have at least one role");
            }

            insert_user(&tx, &user.name, &password.hash, &password.salt)?;

            for role_name in user.role_names.iter() {
                insert_user_role(&tx, &user.name, role_name)?;
            }

            tx.commit()?;
            return Ok(());
        }

        if let Some(ref password) = user.password {
            update_user_password(&tx, &user.name, &password.hash, &password.salt)?;
        };

        if !user.role_names.is_empty() {
            delete_user_roles(&tx, &user.name)?;
            for role_name in user.role_names.iter() {
                insert_user_role(&tx, &user.name, role_name)?;
            }
            // The update of roles will not directly affect the user, so it is necessary to
            // manually trigger an update of the user's update_time here.
            update_user_time(&tx, &user.name)?;
        }

        tx.commit()?;
        Ok(())
    }

    async fn get_user(&self, name: &str) -> Result<Option<User>> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        if !check_user_exists(&tx, name)? {
            return Ok(None);
        }

        let mut user = get_user_detail(&tx, name)?;
        let roles = get_user_roles(&tx, name)?;
        user.role_names = roles.iter().map(|r| r.name.clone()).collect();
        user.roles = Some(roles);

        Ok(Some(user))
    }

    async fn delete_user(&self, name: &str) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        if !check_user_exists(&tx, name)? {
            return Ok(());
        }

        delete_user_roles(&tx, name)?;
        delete_user(&tx, name)?;

        tx.commit()?;
        Ok(())
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        let mut users = list_users(&tx)?;
        for user in users.iter_mut() {
            let role_names = list_user_role_names(&tx, &user.name)?;
            user.role_names = role_names;
        }

        Ok(users)
    }

    async fn validate_user(&self, name: &str, password: &str) -> Result<bool> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        if !check_user_exists(&tx, name)? {
            return Ok(false);
        }

        let correct = get_user_password(&tx, name)?;
        let input_hash = Password::generate_hash(password, &correct.salt);
        Ok(input_hash == correct.hash)
    }

    async fn put_role(&self, role: &Role) -> Result<()> {
        if role.rules.is_empty() {
            bail!("role '{}' has no rules", role.name);
        }

        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        if !check_role_exists(&tx, &role.name)? {
            insert_role(&tx, role)?;
            tx.commit()?;
            return Ok(());
        }

        update_role_rules(&tx, role)?;
        tx.commit()?;
        Ok(())
    }

    async fn get_role(&self, name: &str) -> Result<Option<Role>> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        if !check_role_exists(&tx, name)? {
            return Ok(None);
        }

        let role = get_role(&tx, name)?;
        Ok(Some(role))
    }

    async fn delete_role(&self, name: &str) -> Result<()> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;

        if !check_role_exists(&tx, name)? {
            return Ok(());
        }

        if is_role_in_use(&tx, name)? {
            bail!("role '{}' is in use, cannot be deleted", name);
        }

        delete_role(&tx, name)?;
        tx.commit()?;
        Ok(())
    }

    async fn list_roles(&self) -> Result<Vec<Role>> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction()?;
        list_roles(&tx)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::time::sleep;

    use super::*;
    use crate::types::user::{Password, RoleRule};

    #[tokio::test]
    async fn test_put_user() {
        let store = SqliteStore::memory().unwrap();

        // Create roles first
        let roles = vec![
            Role {
                name: "admin".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["*".to_string()],
                    verbs: vec!["read".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
            Role {
                name: "user".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["documents/*".to_string()],
                    verbs: vec!["read".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
            Role {
                name: "guest".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["public/*".to_string()],
                    verbs: vec!["read".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
        ];

        for role in &roles {
            store.put_role(role).await.unwrap();
        }

        // Test creating new user

        // Case 1: Missing password
        let user1 = User {
            name: "alice".to_string(),
            role_names: vec!["admin".to_string()],
            roles: None,
            password: None,
            create_time: 0,
            update_time: 0,
        };
        assert!(store.put_user(&user1).await.is_err());

        // Case 2: No roles
        let user2 = User {
            name: "alice".to_string(),
            role_names: vec![],
            roles: None,
            password: Some(Password {
                hash: "hash".to_string(),
                salt: "salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        assert!(store.put_user(&user2).await.is_err());

        // Case 3: Non-existent role
        let user3 = User {
            name: "alice".to_string(),
            role_names: vec!["non_existent_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "hash".to_string(),
                salt: "salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        assert!(store.put_user(&user3).await.is_err());

        // Case 4: Successful creation
        let user4 = User {
            name: "alice".to_string(),
            role_names: vec!["admin".to_string()],
            roles: None,
            password: Some(Password {
                hash: "hash".to_string(),
                salt: "salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user4).await.unwrap();

        // Verify user was created with correct password and roles
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            let password_info = get_user_password(&tx, "alice").unwrap();
            assert_eq!(password_info.hash, "hash");
            assert_eq!(password_info.salt, "salt");

            let roles = list_user_role_names(&tx, "alice").unwrap();
            assert_eq!(roles, vec!["admin"]);
        }

        // Case 5: Update password only
        let user5 = User {
            name: "alice".to_string(),
            role_names: vec![],
            roles: None,
            password: Some(Password {
                hash: "new_hash".to_string(),
                salt: "new_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user5).await.unwrap();

        // Verify password was updated but roles remain
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            let password_info = get_user_password(&tx, "alice").unwrap();
            assert_eq!(password_info.hash, "new_hash");
            assert_eq!(password_info.salt, "new_salt");

            let roles = list_user_role_names(&tx, "alice").unwrap();
            assert_eq!(roles, vec!["admin"]);
        }

        // Case 6: Update roles only (with different roles)
        let user6 = User {
            name: "alice".to_string(),
            role_names: vec!["user".to_string(), "guest".to_string()], // Change to different roles
            roles: None,
            password: None,
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user6).await.unwrap();

        // Verify roles were updated and password remains
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            let password_info = get_user_password(&tx, "alice").unwrap();
            assert_eq!(password_info.hash, "new_hash");
            assert_eq!(password_info.salt, "new_salt");

            let roles = list_user_role_names(&tx, "alice").unwrap();
            assert_eq!(roles.len(), 2);
            assert!(roles.contains(&"user".to_string()));
            assert!(roles.contains(&"guest".to_string()));
            assert!(!roles.contains(&"admin".to_string())); // Verify old role was removed
        }

        // Case 7: Update both password and roles (with different roles again)
        let user7 = User {
            name: "alice".to_string(),
            role_names: vec!["admin".to_string(), "guest".to_string()], // Change roles again
            roles: None,
            password: Some(Password {
                hash: "final_hash".to_string(),
                salt: "final_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user7).await.unwrap();

        // Verify both were updated
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            let password_info = get_user_password(&tx, "alice").unwrap();
            assert_eq!(password_info.hash, "final_hash");
            assert_eq!(password_info.salt, "final_salt");

            let roles = list_user_role_names(&tx, "alice").unwrap();
            assert_eq!(roles.len(), 2);
            assert!(roles.contains(&"admin".to_string()));
            assert!(roles.contains(&"guest".to_string()));
            assert!(!roles.contains(&"user".to_string())); // Verify old role was removed
        }
    }

    #[tokio::test]
    async fn test_get_user() {
        let store = SqliteStore::memory().unwrap();

        // Create test roles first
        let roles = vec![
            Role {
                name: "admin".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["*".to_string()],
                    verbs: vec!["read".to_string(), "write".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
            Role {
                name: "user".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["documents/*".to_string()],
                    verbs: vec!["read".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
        ];

        for role in &roles {
            store.put_role(role).await.unwrap();
        }

        // Create a user with multiple roles
        let user = User {
            name: "alice".to_string(),
            role_names: vec!["admin".to_string(), "user".to_string()],
            roles: None,
            password: Some(Password {
                hash: "hash".to_string(),
                salt: "salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user).await.unwrap();

        // Test getting non-existent user
        assert!(store.get_user("non_existent").await.unwrap().is_none());

        // Test getting existing user
        let saved_user = store.get_user("alice").await.unwrap().unwrap();

        // Verify basic user info
        assert_eq!(saved_user.name, "alice");
        assert!(saved_user.create_time > 0);
        assert_eq!(saved_user.create_time, saved_user.update_time);
        assert!(saved_user.password.is_none());

        // Verify role names
        assert_eq!(saved_user.role_names.len(), 2);
        assert!(saved_user.role_names.contains(&"admin".to_string()));
        assert!(saved_user.role_names.contains(&"user".to_string()));

        // Verify complete role information
        let saved_roles = saved_user.roles.unwrap();
        assert_eq!(saved_roles.len(), 2);

        for role in saved_roles {
            let original = roles.iter().find(|r| r.name == role.name).unwrap();
            assert_eq!(role.rules, original.rules);
            assert!(role.create_time > 0);
            assert_eq!(role.create_time, role.update_time);
        }
    }

    #[tokio::test]
    async fn test_delete_user() {
        let store = SqliteStore::memory().unwrap();

        // Create test role first
        let role = Role {
            name: "admin".to_string(),
            rules: vec![RoleRule {
                objects: vec!["*".to_string()],
                verbs: vec!["read".to_string()],
            }],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&role).await.unwrap();

        // Create test user
        let user = User {
            name: "alice".to_string(),
            role_names: vec!["admin".to_string()],
            roles: None,
            password: Some(Password {
                hash: "hash".to_string(),
                salt: "salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user).await.unwrap();

        // Verify user exists with role
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            assert!(check_user_exists(&tx, "alice").unwrap());
            let roles = list_user_role_names(&tx, "alice").unwrap();
            assert_eq!(roles, vec!["admin"]);
        }

        // Test deleting non-existent user (should succeed)
        store.delete_user("non_existent").await.unwrap();

        // Delete user
        store.delete_user("alice").await.unwrap();

        // Verify user and roles are deleted
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            // User should not exist
            assert!(!check_user_exists(&tx, "alice").unwrap());

            // User roles should be deleted
            let roles = list_user_role_names(&tx, "alice").unwrap();
            assert!(roles.is_empty());

            // User password should be deleted
            assert!(get_user_password(&tx, "alice").is_err());

            // Role should still exist
            assert!(check_role_exists(&tx, "admin").unwrap());
        }

        // Test deleting already deleted user (should succeed)
        store.delete_user("alice").await.unwrap();
    }

    #[tokio::test]
    async fn test_list_users() {
        let store = SqliteStore::memory().unwrap();

        // Test empty list
        let users = store.list_users().await.unwrap();
        assert!(users.is_empty());

        // Create test roles first
        let roles = vec![
            Role {
                name: "admin".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["*".to_string()],
                    verbs: vec!["read".to_string(), "write".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
            Role {
                name: "user".to_string(),
                rules: vec![RoleRule {
                    objects: vec!["documents/*".to_string()],
                    verbs: vec!["read".to_string()],
                }],
                create_time: 0,
                update_time: 0,
            },
        ];

        for role in &roles {
            store.put_role(role).await.unwrap();
        }

        // Create multiple users
        let users = vec![
            User {
                name: "alice".to_string(),
                role_names: vec!["admin".to_string(), "user".to_string()],
                roles: None,
                password: Some(Password {
                    hash: "hash1".to_string(),
                    salt: "salt1".to_string(),
                }),
                create_time: 0,
                update_time: 0,
            },
            User {
                name: "bob".to_string(),
                role_names: vec!["user".to_string()],
                roles: None,
                password: Some(Password {
                    hash: "hash2".to_string(),
                    salt: "salt2".to_string(),
                }),
                create_time: 0,
                update_time: 0,
            },
        ];

        // Create users with delay to ensure different timestamps
        for user in &users {
            sleep(Duration::from_secs(1)).await;
            store.put_user(user).await.unwrap();
        }

        // Verify initial list (most recent first)
        let saved_users = store.list_users().await.unwrap();
        assert_eq!(saved_users.len(), 2);
        assert_eq!(saved_users[0].name, "bob"); // Created last
        assert_eq!(saved_users[1].name, "alice"); // Created first

        // Verify timestamps are in descending order
        assert!(saved_users[0].update_time > saved_users[1].update_time);

        // Verify role information
        assert_eq!(saved_users[0].role_names, vec!["user"]);
        assert_eq!(saved_users[1].role_names.len(), 2);
        assert!(saved_users[1].role_names.contains(&"admin".to_string()));
        assert!(saved_users[1].role_names.contains(&"user".to_string()));

        // Update a user and verify new order
        sleep(Duration::from_secs(1)).await;

        let updated_user = User {
            name: "alice".to_string(),
            role_names: vec!["admin".to_string()],
            roles: None,
            password: None,
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&updated_user).await.unwrap();

        // Verify updated list
        let saved_users = store.list_users().await.unwrap();
        assert_eq!(saved_users.len(), 2);
        assert_eq!(saved_users[0].name, "alice"); // Updated last
        assert_eq!(saved_users[1].name, "bob");

        // Verify timestamps are still in descending order
        assert!(saved_users[0].update_time > saved_users[1].update_time);

        // Verify role information is updated
        assert_eq!(saved_users[0].role_names, vec!["admin"]);
        assert_eq!(saved_users[1].role_names, vec!["user"]);

        // Verify password information is not included
        for user in &saved_users {
            assert!(user.password.is_none());
            assert!(user.roles.is_none());
        }
    }

    #[tokio::test]
    async fn test_validate_user() {
        let store = SqliteStore::memory().unwrap();

        // Create test role
        let role = Role {
            name: "admin".to_string(),
            rules: vec![RoleRule {
                objects: vec!["*".to_string()],
                verbs: vec!["read".to_string()],
            }],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&role).await.unwrap();

        // Test data
        let name = "alice";
        let password = "my_password";
        let salt = "random_salt";
        let hash = Password::generate_hash(password, salt);

        // Create user
        let user = User {
            name: name.to_string(),
            role_names: vec!["admin".to_string()],
            roles: None,
            password: Some(Password {
                hash: hash.clone(),
                salt: salt.to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user).await.unwrap();

        // Test validation with correct password
        assert!(store.validate_user(name, password).await.unwrap());

        // Test validation with wrong password
        assert!(!store.validate_user(name, "wrong_password").await.unwrap());

        // Test validation with non-existent user
        assert!(!store.validate_user("non_existent", password).await.unwrap());

        // Update password and verify
        let new_password = "new_password";
        let new_salt = "new_salt";
        let new_hash = Password::generate_hash(new_password, new_salt);

        let updated_user = User {
            name: name.to_string(),
            role_names: vec![],
            roles: None,
            password: Some(Password {
                hash: new_hash,
                salt: new_salt.to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&updated_user).await.unwrap();

        // Test validation with old password (should fail)
        assert!(!store.validate_user(name, password).await.unwrap());

        // Test validation with new password (should succeed)
        assert!(store.validate_user(name, new_password).await.unwrap());
    }

    #[tokio::test]
    async fn test_put_role() {
        let store = SqliteStore::memory().unwrap();

        // Test data
        let role_name = "admin";
        let initial_rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string(), "write".to_string()],
        };

        // Case 1: Create role with empty rules (should fail)
        let empty_role = Role {
            name: role_name.to_string(),
            rules: vec![],
            create_time: 0,
            update_time: 0,
        };
        assert!(store.put_role(&empty_role).await.is_err());

        // Case 2: Create role successfully
        let role = Role {
            name: role_name.to_string(),
            rules: vec![initial_rule.clone()],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&role).await.unwrap();

        // Verify role was created correctly
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            assert!(check_role_exists(&tx, role_name).unwrap());
            let saved_role = get_role(&tx, role_name).unwrap();
            assert_eq!(saved_role.name, role_name);
            assert_eq!(saved_role.rules, vec![initial_rule.clone()]);
            assert!(saved_role.create_time > 0);
            assert_eq!(saved_role.create_time, saved_role.update_time);
        }

        // Case 3: Update role with empty rules (should fail)
        let empty_update = Role {
            name: role_name.to_string(),
            rules: vec![],
            create_time: 0,
            update_time: 0,
        };
        assert!(store.put_role(&empty_update).await.is_err());

        // Sleep to ensure different timestamp
        sleep(Duration::from_secs(1)).await;

        // Case 4: Update role with new rules
        let new_rule = RoleRule {
            objects: vec!["documents/*".to_string()],
            verbs: vec!["read".to_string()],
        };
        let updated_role = Role {
            name: role_name.to_string(),
            rules: vec![new_rule.clone()],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&updated_role).await.unwrap();

        // Verify role was updated correctly
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            let saved_role = get_role(&tx, role_name).unwrap();
            assert_eq!(saved_role.name, role_name);
            assert_eq!(saved_role.rules, vec![new_rule]);
            assert!(saved_role.update_time > saved_role.create_time);
        }

        // Case 5: Update role with multiple rules
        let multiple_rules = vec![
            RoleRule {
                objects: vec!["documents/*".to_string()],
                verbs: vec!["read".to_string()],
            },
            RoleRule {
                objects: vec!["images/*".to_string()],
                verbs: vec!["write".to_string()],
            },
        ];
        let multi_rule_role = Role {
            name: role_name.to_string(),
            rules: multiple_rules.clone(),
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&multi_rule_role).await.unwrap();

        // Verify multiple rules were saved correctly
        {
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();

            let saved_role = get_role(&tx, role_name).unwrap();
            assert_eq!(saved_role.name, role_name);
            assert_eq!(saved_role.rules, multiple_rules);
        }
    }

    #[tokio::test]
    async fn test_get_role() {
        let store = SqliteStore::memory().unwrap();

        // Test data
        let role_name = "admin";
        let rules = vec![
            RoleRule {
                objects: vec!["*".to_string()],
                verbs: vec!["read".to_string(), "write".to_string()],
            },
            RoleRule {
                objects: vec!["documents/*".to_string()],
                verbs: vec!["delete".to_string()],
            },
        ];

        // Test getting non-existent role
        assert!(store.get_role(role_name).await.unwrap().is_none());

        // Create role
        let role = Role {
            name: role_name.to_string(),
            rules: rules.clone(),
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&role).await.unwrap();

        // Test getting existing role
        let saved_role = store.get_role(role_name).await.unwrap().unwrap();

        // Verify role details
        assert_eq!(saved_role.name, role_name);
        assert_eq!(saved_role.rules, rules);
        assert!(saved_role.create_time > 0);
        assert_eq!(saved_role.create_time, saved_role.update_time);

        // Update role and verify changes
        sleep(Duration::from_secs(1)).await;

        let new_rules = vec![RoleRule {
            objects: vec!["new/*".to_string()],
            verbs: vec!["read".to_string()],
        }];
        let updated_role = Role {
            name: role_name.to_string(),
            rules: new_rules.clone(),
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&updated_role).await.unwrap();

        // Get and verify updated role
        let saved_role = store.get_role(role_name).await.unwrap().unwrap();
        assert_eq!(saved_role.name, role_name);
        assert_eq!(saved_role.rules, new_rules);
        assert!(saved_role.update_time > saved_role.create_time);

        // Delete role and verify it's gone
        store.delete_role(role_name).await.unwrap();
        assert!(store.get_role(role_name).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_role() {
        let store = SqliteStore::memory().unwrap();

        // Test data
        let role_name = "admin";
        let rule = RoleRule {
            objects: vec!["*".to_string()],
            verbs: vec!["read".to_string(), "write".to_string()],
        };

        // Test deleting non-existent role (should succeed)
        assert!(store.delete_role(role_name).await.is_ok());

        // Create role
        let role = Role {
            name: role_name.to_string(),
            rules: vec![rule],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&role).await.unwrap();

        // Verify role exists
        assert!(store.get_role(role_name).await.unwrap().is_some());

        // Create user and assign role
        let user = User {
            name: "alice".to_string(),
            role_names: vec![role_name.to_string()],
            roles: None,
            password: Some(Password {
                hash: "hash".to_string(),
                salt: "salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        store.put_user(&user).await.unwrap();

        // Try to delete role while in use (should fail)
        assert!(store.delete_role(role_name).await.is_err());

        // Verify role still exists
        assert!(store.get_role(role_name).await.unwrap().is_some());

        // Delete user
        store.delete_user("alice").await.unwrap();

        // Now delete role (should succeed)
        store.delete_role(role_name).await.unwrap();

        // Verify role is deleted
        assert!(store.get_role(role_name).await.unwrap().is_none());

        // Try to delete already deleted role (should succeed)
        assert!(store.delete_role(role_name).await.is_ok());
    }

    #[tokio::test]
    async fn test_list_roles() {
        let store = SqliteStore::memory().unwrap();

        // Test empty list
        let roles = store.list_roles().await.unwrap();
        assert!(roles.is_empty());

        // Create multiple roles
        let role_rules = [
            RoleRule {
                objects: vec!["*".to_string()],
                verbs: vec!["read".to_string(), "write".to_string()],
            },
            RoleRule {
                objects: vec!["documents/*".to_string()],
                verbs: vec!["read".to_string()],
            },
        ];

        let roles = vec![
            Role {
                name: "admin".to_string(),
                rules: vec![role_rules[0].clone()],
                create_time: 0,
                update_time: 0,
            },
            Role {
                name: "user".to_string(),
                rules: vec![role_rules[1].clone()],
                create_time: 0,
                update_time: 0,
            },
        ];

        // Create roles with delay to ensure different timestamps
        for role in &roles {
            sleep(Duration::from_secs(1)).await;
            store.put_role(role).await.unwrap();
        }

        // Verify initial list (most recent first)
        let saved_roles = store.list_roles().await.unwrap();
        assert_eq!(saved_roles.len(), 2);
        assert_eq!(saved_roles[0].name, "user"); // Created last
        assert_eq!(saved_roles[1].name, "admin"); // Created first

        // Verify timestamps are in descending order
        assert!(saved_roles[0].update_time > saved_roles[1].update_time);

        // Verify rules are preserved
        assert_eq!(saved_roles[0].rules, vec![role_rules[1].clone()]);
        assert_eq!(saved_roles[1].rules, vec![role_rules[0].clone()]);

        // Update a role and verify new order
        sleep(Duration::from_secs(1)).await;

        let updated_role = Role {
            name: "admin".to_string(),
            rules: vec![RoleRule {
                objects: vec!["new/*".to_string()],
                verbs: vec!["read".to_string()],
            }],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&updated_role).await.unwrap();

        // Verify updated list
        let saved_roles = store.list_roles().await.unwrap();
        assert_eq!(saved_roles.len(), 2);
        assert_eq!(saved_roles[0].name, "admin"); // Updated last
        assert_eq!(saved_roles[1].name, "user");

        // Verify timestamps are still in descending order
        assert!(saved_roles[0].update_time > saved_roles[1].update_time);

        // Verify rules are updated
        assert_eq!(saved_roles[0].rules, updated_role.rules);
        assert_eq!(saved_roles[1].rules, vec![role_rules[1].clone()]);

        // Delete a role and verify list
        store.delete_role("user").await.unwrap();

        let saved_roles = store.list_roles().await.unwrap();
        assert_eq!(saved_roles.len(), 1);
        assert_eq!(saved_roles[0].name, "admin");
    }

    #[tokio::test]
    async fn test_concurrent() {
        let store = Arc::new(SqliteStore::memory().unwrap());

        // Create a role first
        let role = Role {
            name: "user".to_string(),
            rules: vec![RoleRule {
                objects: vec!["documents/*".to_string()],
                verbs: vec!["read".to_string()],
            }],
            create_time: 0,
            update_time: 0,
        };
        store.put_role(&role).await.unwrap();

        // Prepare concurrent tasks for creating users
        let num_users = 100;

        let handles: Vec<_> = (0..num_users)
            .map(|i| {
                let store = Arc::clone(&store);
                let user = User {
                    name: format!("user{}", i),
                    role_names: vec!["user".to_string()],
                    roles: None,
                    password: Some(Password {
                        hash: format!("hash{}", i),
                        salt: format!("salt{}", i),
                    }),
                    create_time: 0,
                    update_time: 0,
                };

                tokio::spawn(async move {
                    store.put_user(&user).await.unwrap();
                })
            })
            .collect();

        // Wait for all creation tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all users were created
        let users = store.list_users().await.unwrap();
        assert_eq!(users.len(), num_users);

        // Verify each user exists with correct data
        for i in 0..num_users {
            let name = format!("user{}", i);
            let user = store.get_user(&name).await.unwrap().unwrap();

            assert_eq!(user.name, name);
            assert_eq!(user.role_names, vec!["user"]);

            // Verify password info
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();
            let password = get_user_password(&tx, &name).unwrap();
            assert_eq!(password.hash, format!("hash{}", i));
            assert_eq!(password.salt, format!("salt{}", i));
        }

        // Now delete all users concurrently
        let handles: Vec<_> = (0..num_users)
            .map(|i| {
                let store = Arc::clone(&store);
                let name = format!("user{}", i);

                tokio::spawn(async move {
                    store.delete_user(&name).await.unwrap();
                })
            })
            .collect();

        // Wait for all deletion tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all users were deleted
        let users = store.list_users().await.unwrap();
        assert!(users.is_empty());

        // Verify each user no longer exists
        for i in 0..num_users {
            let name = format!("user{}", i);
            assert!(store.get_user(&name).await.unwrap().is_none());

            // Verify user data is gone
            let mut conn = store.conn.lock().await;
            let tx = conn.transaction().unwrap();
            assert!(!check_user_exists(&tx, &name).unwrap());
            assert!(get_user_password(&tx, &name).is_err());
            assert!(list_user_role_names(&tx, &name).unwrap().is_empty());
        }
    }
}
