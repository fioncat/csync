use std::collections::{HashMap, HashSet};

use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::display::TerminalDisplay;
use crate::time::format_since;

/// User information with RBAC roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Username, must be unique
    #[serde(default = "default_string")]
    pub name: String,

    /// User creation timestamp
    #[serde(default = "default_time")]
    pub create_time: u64,

    /// User last update timestamp
    #[serde(default = "default_time")]
    pub update_time: u64,

    /// List of roles assigned to this user
    #[serde(default = "default_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,

    /// Optional password field, only used during user creation or password update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Role definition for RBAC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Role name, must be unique
    #[serde(default = "default_string")]
    pub name: String,

    /// List of rules that define the permissions of this role
    #[serde(default = "default_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RoleRule>,

    /// Role creation timestamp
    #[serde(default = "default_time")]
    pub create_time: u64,

    /// Role last update timestamp
    #[serde(default = "default_time")]
    pub update_time: u64,
}

/// Rule that defines what resources can be accessed with what operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoleRule {
    /// Set of resources this rule applies to.
    /// Use "*" to allow access to all resources.
    pub resources: HashSet<String>,

    /// Set of allowed operations on these resources.
    /// Available verbs: "put", "get", "head", "list", "delete".
    /// Use "*" to allow all operations.
    pub verbs: HashSet<String>,
}

/// Response for whoami request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoamiResponse {
    /// Current authenticated username
    pub name: String,
}

/// Response for authorization check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaniResponse {
    /// Whether the operation is allowed
    pub allow: bool,
}

impl User {
    /// Length of the random salt used for password hashing
    const SALT_LENGTH: usize = 30;

    /// Generates a password hash with a random salt if password is set
    ///
    /// # Returns
    /// - Some((hash, salt)): Password hash and the salt used
    /// - None: If no password is set
    pub fn generate_password_hash(&self) -> Option<(String, String)> {
        match self.password {
            Some(ref password) => {
                let salt = Self::generate_salt(Self::SALT_LENGTH);
                let hash = Self::get_password_hash(password, &salt);
                Some((hash, salt))
            }
            None => None,
        }
    }

    /// Generates a password hash using the provided salt
    ///
    /// The hash is generated using SHA256(password + salt)
    pub fn get_password_hash(password: &str, salt: &str) -> String {
        let combined = format!("{password}{salt}");
        let hash = Sha256::digest(combined.as_bytes());
        format!("{:x}", hash)
    }

    /// Generates a random salt string of specified length
    fn generate_salt(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::rng();

        (0..length)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}

impl TerminalDisplay for User {
    fn table_titles() -> Vec<&'static str> {
        vec!["Name", "Create", "Update"]
    }

    fn table_row(self) -> Vec<String> {
        vec![
            self.name,
            format_since(self.create_time),
            format_since(self.update_time),
        ]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec!["name", "create_time", "update_time"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        vec![
            ("name", self.name),
            ("create_time", self.create_time.to_string()),
            ("update_time", self.update_time.to_string()),
        ]
        .into_iter()
        .collect()
    }
}

impl TerminalDisplay for Role {
    fn table_titles() -> Vec<&'static str> {
        vec!["Name", "Create", "Update"]
    }

    fn table_row(self) -> Vec<String> {
        vec![
            self.name,
            format_since(self.create_time),
            format_since(self.update_time),
        ]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec!["name", "create_time", "update_time"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        vec![
            ("name", self.name),
            ("create_time", self.create_time.to_string()),
            ("update_time", self.update_time.to_string()),
        ]
        .into_iter()
        .collect()
    }
}

fn default_time() -> u64 {
    0
}

fn default_vec<T>() -> Vec<T> {
    Vec::new()
}

fn default_string() -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password() {
        // Test password hash generation
        let user = User {
            name: "test".to_string(),
            password: Some("mypassword123".to_string()),
            create_time: 0,
            update_time: 0,
            roles: vec![],
        };

        // Test generate_password_hash
        let hash_result = user.generate_password_hash();
        assert!(hash_result.is_some());
        let (hash1, salt1) = hash_result.unwrap();

        // Verify salt length
        assert_eq!(salt1.len(), User::SALT_LENGTH);

        // Verify salt charset
        assert!(salt1.chars().all(|c| c.is_ascii_alphanumeric()));

        // Test get_password_hash consistency
        let hash2 = User::get_password_hash(user.password.as_ref().unwrap(), &salt1);
        assert_eq!(hash1, hash2);

        // Test different salts produce different hashes
        let salt2 = User::generate_salt(User::SALT_LENGTH);
        assert_ne!(salt1, salt2);
        let hash3 = User::get_password_hash(&user.password.unwrap(), &salt2);
        assert_ne!(hash1, hash3);

        // Test user without password
        let user_no_pwd = User {
            name: "test".to_string(),
            password: None,
            create_time: 0,
            update_time: 0,
            roles: vec![],
        };
        assert!(user_no_pwd.generate_password_hash().is_none());
    }
}
