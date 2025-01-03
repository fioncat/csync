use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub role_names: Vec<String>,
    pub create_time: u64,
    pub update_time: u64,

    pub roles: Option<Vec<Role>>,
    pub password: Option<Password>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub rules: Vec<RoleRule>,

    pub create_time: u64,
    pub update_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoleRule {
    pub objects: Vec<String>,
    pub verbs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Password {
    pub salt: String,
    pub hash: String,
}

impl Password {
    pub fn generate_hash(password: &str, salt: &str) -> String {
        let mut input = String::from(password);
        input.push_str(salt);
        sha256::digest(input)
    }
}
