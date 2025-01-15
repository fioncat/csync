use std::collections::{HashMap, HashSet};

use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::display::TerminalDisplay;
use crate::time::format_since;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(default = "default_string")]
    pub name: String,

    #[serde(default = "default_time")]
    pub create_time: u64,

    #[serde(default = "default_time")]
    pub update_time: u64,

    #[serde(default = "default_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Role>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    #[serde(default = "default_string")]
    pub name: String,

    #[serde(default = "default_vec")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<RoleRule>,

    #[serde(default = "default_time")]
    pub create_time: u64,

    #[serde(default = "default_time")]
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRule {
    pub resources: HashSet<String>,
    pub verbs: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoamiResponse {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaniResponse {
    pub allow: bool,
}

impl User {
    const SALT_LENGTH: usize = 30;

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

    pub fn get_password_hash(password: &str, salt: &str) -> String {
        let combined = format!("{password}{salt}");
        let hash = Sha256::digest(combined.as_bytes());
        format!("{:x}", hash)
    }

    fn generate_salt(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();

        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
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
