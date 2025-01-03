use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use tokio::sync::Mutex;

use crate::types::user::{Role, User};

use super::Storage;

pub struct Cache {
    users: Mutex<RefCell<HashMap<String, CacheUser>>>,
    upstream: Arc<Box<dyn Storage>>,
    expiry: usize,
}

struct CacheUser {
    user: Option<User>,
    password: Option<String>,
    expiry: usize,
}

impl Cache {
    pub fn new(upstream: Arc<Box<dyn Storage>>, expiry: usize) -> Self {
        Self {
            users: Mutex::new(RefCell::new(HashMap::new())),
            upstream,
            expiry,
        }
    }

    fn get_expiry(&self) -> usize {
        if self.expiry == 0 {
            return 0;
        }
        Local::now().timestamp() as usize + self.expiry
    }
}

impl CacheUser {
    fn is_expired(&self) -> bool {
        if self.expiry == 0 {
            return false;
        }
        let now = Local::now().timestamp() as usize;
        now >= self.expiry
    }
}

#[async_trait::async_trait]
impl Storage for Cache {
    async fn put_user(&self, user: &User) -> Result<()> {
        self.upstream.put_user(user).await?;

        let users = self.users.lock().await;
        users.borrow_mut().remove(&user.name);

        Ok(())
    }

    async fn get_user(&self, name: &str) -> Result<Option<User>> {
        let users = self.users.lock().await;

        if let Some(cache_user) = users.borrow().get(name) {
            if !cache_user.is_expired() {
                // cache hit
                return Ok(cache_user.user.clone());
            }
        }

        let user = self.upstream.get_user(name).await?;
        users.borrow_mut().insert(
            name.to_string(),
            CacheUser {
                user: user.clone(),
                password: None,
                expiry: self.get_expiry(),
            },
        );

        Ok(user)
    }

    async fn delete_user(&self, name: &str) -> Result<()> {
        self.upstream.delete_user(name).await?;

        let users = self.users.lock().await;
        users.borrow_mut().insert(
            name.to_string(),
            CacheUser {
                user: None,
                password: None,
                expiry: self.get_expiry(),
            },
        );

        Ok(())
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        self.upstream.list_users().await
    }

    async fn validate_user(&self, name: &str, password: &str) -> Result<bool> {
        let users = self.users.lock().await;

        let mut user_info = None;
        if let Some(cache_user) = users.borrow().get(name) {
            if !cache_user.is_expired() {
                if cache_user.user.is_none() {
                    return Ok(false);
                }

                if let Some(ref cache_password) = cache_user.password {
                    return Ok(cache_password == password);
                }

                user_info = cache_user.user.clone();
            }
        }

        if !self.upstream.validate_user(name, password).await? {
            return Ok(false);
        }

        if user_info.is_none() {
            user_info = self.upstream.get_user(name).await?;
        }

        users.borrow_mut().insert(
            name.to_string(),
            CacheUser {
                user: user_info,
                password: Some(password.to_string()),
                expiry: self.get_expiry(),
            },
        );

        Ok(true)
    }

    async fn put_role(&self, role: &Role) -> Result<()> {
        self.upstream.put_role(role).await
    }

    async fn get_role(&self, name: &str) -> Result<Option<Role>> {
        self.upstream.get_role(name).await
    }

    async fn delete_role(&self, name: &str) -> Result<()> {
        self.upstream.delete_role(name).await
    }

    async fn list_roles(&self) -> Result<Vec<Role>> {
        self.upstream.list_roles().await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use crate::types::user::Password;

    use super::*;

    struct MockStore {
        user: Mutex<RefCell<Option<User>>>,
    }

    impl MockStore {
        fn new() -> Self {
            Self {
                user: Mutex::new(RefCell::new(None)),
            }
        }
    }

    #[async_trait::async_trait]
    impl Storage for MockStore {
        async fn get_user(&self, name: &str) -> Result<Option<User>> {
            if name == "nonexistent" {
                return Ok(None);
            }
            let guard = self.user.lock().await;
            let user = guard.borrow().clone();
            Ok(user)
        }

        async fn put_user(&self, user: &User) -> Result<()> {
            let guard = self.user.lock().await;
            *guard.borrow_mut() = Some(user.clone());
            Ok(())
        }

        async fn delete_user(&self, _name: &str) -> Result<()> {
            let guard = self.user.lock().await;
            *guard.borrow_mut() = None;
            Ok(())
        }

        async fn list_users(&self) -> Result<Vec<User>> {
            let guard = self.user.lock().await;
            let users = guard.borrow().clone().map(|u| vec![u]).unwrap_or_default();
            Ok(users)
        }

        async fn validate_user(&self, _name: &str, _password: &str) -> Result<bool> {
            Ok(true)
        }

        async fn put_role(&self, _role: &Role) -> Result<()> {
            Ok(())
        }

        async fn get_role(&self, _name: &str) -> Result<Option<Role>> {
            Ok(None)
        }

        async fn delete_role(&self, _name: &str) -> Result<()> {
            Ok(())
        }

        async fn list_roles(&self) -> Result<Vec<Role>> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_user_get() -> Result<()> {
        let mock_store = Arc::new(Box::new(MockStore::new()) as Box<dyn Storage>);
        let cache = Cache::new(Arc::clone(&mock_store), 60);

        // 1. Create and insert initial cache_user
        let cache_user = User {
            name: "test_user".to_string(),
            role_names: vec!["cache_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "cache_hash".to_string(),
                salt: "cache_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        cache.put_user(&cache_user).await?;

        // 2. Get user twice to verify cache is working
        let result1 = cache.get_user("test_user").await?;
        assert_eq!(
            result1.as_ref().map(|u| &u.role_names),
            Some(&cache_user.role_names)
        );

        let result2 = cache.get_user("test_user").await?;
        assert_eq!(
            result2.as_ref().map(|u| &u.role_names),
            Some(&cache_user.role_names)
        );

        // 3. Update user directly in mock_store (bypassing cache)
        let updated_user = User {
            name: "test_user".to_string(),
            role_names: vec!["updated_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "updated_hash".to_string(),
                salt: "updated_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        mock_store.put_user(&updated_user).await?;

        // 4. Get through cache should still return cache_user
        let result3 = cache.get_user("test_user").await?;
        assert_eq!(
            result3.as_ref().map(|u| &u.role_names),
            Some(&cache_user.role_names)
        );
        assert_ne!(
            result3.as_ref().map(|u| &u.role_names),
            Some(&updated_user.role_names)
        );

        // 5. Update through cache
        let final_user = User {
            name: "test_user".to_string(),
            role_names: vec!["final_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "final_hash".to_string(),
                salt: "final_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        cache.put_user(&final_user).await?;

        // Verify both cache and store are updated
        let cache_result = cache.get_user("test_user").await?;
        let store_result = mock_store.get_user("test_user").await?;

        assert_eq!(
            cache_result.as_ref().map(|u| &u.role_names),
            Some(&final_user.role_names)
        );
        assert_eq!(
            store_result.as_ref().map(|u| &u.role_names),
            Some(&final_user.role_names)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_user_cache_expiry() -> Result<()> {
        // Create cache with 1 second expiration time
        let mock_store = Arc::new(Box::new(MockStore::new()) as Box<dyn Storage>);
        let cache = Cache::new(Arc::clone(&mock_store), 1); // 1 second expiry

        // Create and insert initial user
        let initial_user = User {
            name: "test_user".to_string(),
            role_names: vec!["initial_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "initial_hash".to_string(),
                salt: "initial_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        cache.put_user(&initial_user).await?;

        // First get should return initial user
        let result1 = cache.get_user("test_user").await?;
        assert_eq!(
            result1.as_ref().map(|u| &u.role_names),
            Some(&initial_user.role_names)
        );

        // Update store directly with new user
        let updated_user = User {
            name: "test_user".to_string(),
            role_names: vec!["updated_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "updated_hash".to_string(),
                salt: "updated_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        mock_store.put_user(&updated_user).await?;

        // Immediate get should still return initial user (from cache)
        let result2 = cache.get_user("test_user").await?;
        assert_eq!(
            result2.as_ref().map(|u| &u.role_names),
            Some(&initial_user.role_names)
        );

        // Wait for cache to expire (2 seconds to be safe)
        sleep(Duration::from_secs(2)).await;

        // After expiry, should get updated user from store
        let result3 = cache.get_user("test_user").await?;
        assert_eq!(
            result3.as_ref().map(|u| &u.role_names),
            Some(&updated_user.role_names)
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_user() -> Result<()> {
        let mock_store = Arc::new(Box::new(MockStore::new()) as Box<dyn Storage>);
        let cache = Cache::new(Arc::clone(&mock_store), 60);

        // Create initial user
        let test_user = User {
            name: "test_user".to_string(),
            role_names: vec!["test_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "correct_hash".to_string(),
                salt: "test_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        mock_store.put_user(&test_user).await?;

        // First validation with correct password should succeed and cache the result
        let result1 = cache.validate_user("test_user", "correct_password").await?;
        assert!(result1);

        // Second validation with wrong password should fail (using cached result)
        let result2 = cache.validate_user("test_user", "wrong_password").await?;
        assert!(!result2);

        let result3 = cache.validate_user("test_user", "correct_password").await?;
        assert!(result3);

        // Even though mock_store always returns true, cache should use cached password
        assert!(
            mock_store
                .validate_user("test_user", "wrong_password")
                .await?
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_user_expiry() -> Result<()> {
        // Create cache with 1 second expiration time
        let mock_store = Arc::new(Box::new(MockStore::new()) as Box<dyn Storage>);
        let cache = Cache::new(Arc::clone(&mock_store), 1);

        // Create initial user
        let test_user = User {
            name: "test_user".to_string(),
            role_names: vec!["test_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "initial_hash".to_string(),
                salt: "initial_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        mock_store.put_user(&test_user).await?;

        // First validation should succeed and cache the password
        let result1 = cache.validate_user("test_user", "correct_password").await?;
        assert!(result1);

        // Wrong password should fail using cached result
        let result2 = cache.validate_user("test_user", "wrong_password").await?;
        assert!(!result2);

        // Wait for cache to expire
        sleep(Duration::from_secs(2)).await;

        // After expiry, validation should succeed because mock_store always returns true
        let result3 = cache.validate_user("test_user", "wrong_password").await?;
        assert!(result3);

        Ok(())
    }

    #[tokio::test]
    async fn test_validate_user_nonexistent() -> Result<()> {
        let mock_store = Arc::new(Box::new(MockStore::new()) as Box<dyn Storage>);
        let cache = Cache::new(Arc::clone(&mock_store), 60);

        // First get nonexistent user to cache the None result
        let result = cache.get_user("nonexistent").await?;
        assert!(result.is_none());

        // Validate should return false because user is cached as nonexistent
        let validate_result = cache.validate_user("nonexistent", "any_password").await?;
        assert!(!validate_result);

        // Even though mock_store would return true, cache should use the cached None result
        assert!(
            mock_store
                .validate_user("nonexistent", "any_password")
                .await?
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_get_user_after_validate() -> Result<()> {
        let mock_store = Arc::new(Box::new(MockStore::new()) as Box<dyn Storage>);
        let cache = Cache::new(Arc::clone(&mock_store), 60);

        // Create initial user in store
        let test_user = User {
            name: "test_user".to_string(),
            role_names: vec!["test_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "test_hash".to_string(),
                salt: "test_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        mock_store.put_user(&test_user).await?;

        // Validate user first (this should cache the user info)
        let validate_result = cache.validate_user("test_user", "correct_password").await?;
        assert!(validate_result);

        // Update user directly in store with different data
        let updated_user = User {
            name: "test_user".to_string(),
            role_names: vec!["updated_role".to_string()],
            roles: None,
            password: Some(Password {
                hash: "updated_hash".to_string(),
                salt: "updated_salt".to_string(),
            }),
            create_time: 0,
            update_time: 0,
        };
        mock_store.put_user(&updated_user).await?;

        // Get user should return cached data from validate, not updated data from store
        let user_result = cache.get_user("test_user").await?;
        assert!(user_result.is_some());

        let cached_user = user_result.unwrap();
        assert_eq!(cached_user.name, test_user.name);
        assert_eq!(cached_user.role_names, test_user.role_names);
        assert_ne!(cached_user.role_names, updated_user.role_names);

        // Verify store has different data
        let store_user = mock_store.get_user("test_user").await?.unwrap();
        assert_eq!(store_user.role_names, updated_user.role_names);

        Ok(())
    }
}
