use std::collections::HashMap;

use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::display::TerminalDisplay;
use crate::{parse_from_map, time};

use super::{QueryRequest, Request, RequestField, Value};

pub const GET_TOKEN_PATH: &str = "/v1/token";
pub const USER_PATH: &str = "/v1/user";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct User {
    pub name: String,

    pub admin: bool,

    pub update_time: u64,
}

impl TerminalDisplay for User {
    fn table_titles() -> Vec<&'static str> {
        vec!["Name", "Admin", "Update"]
    }

    fn table_row(self) -> Vec<String> {
        let update_time = time::format_time(self.update_time);
        vec![self.name, self.admin.to_string(), update_time]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec!["name", "admin", "update_time"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        let update_time = time::format_time(self.update_time);
        let mut row = HashMap::new();
        row.insert("name", self.name);
        row.insert("admin", self.admin.to_string());
        row.insert("update_time", update_time);
        row
    }
}

#[derive(Debug, Default)]
pub struct PutUserRequest {
    pub name: String,
    pub password: String,
    pub admin: bool,
}

impl Request for PutUserRequest {
    fn fields(self) -> Vec<RequestField> {
        vec![
            RequestField {
                name: "name",
                value: Value::Text(self.name.clone()),
            },
            RequestField {
                name: "password",
                value: Value::Text(self.password.clone()),
            },
            RequestField {
                name: "admin",
                value: Value::Bool(self.admin),
            },
        ]
    }

    fn complete(&mut self, mut fields: HashMap<String, String>) -> Result<()> {
        self.name = fields.remove("name").unwrap_or_default();
        if self.name.is_empty() {
            bail!("name is required to put user");
        }
        if self.name == "admin" {
            bail!("name cannot be 'admin'");
        }
        if !is_valid_name(&self.name) {
            bail!("invalid name");
        }

        self.password = fields.remove("password").unwrap_or_default();
        if self.password.is_empty() {
            bail!("password is required to put user");
        }
        self.admin = parse_from_map!(fields, "admin").unwrap_or_default();
        Ok(())
    }
}

static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]+$").unwrap());

fn is_valid_name(name: &str) -> bool {
    NAME_REGEX.is_match(name)
}

#[derive(Debug, Default, Clone)]
pub struct GetUserRequest {
    pub name: Option<String>,

    pub query: QueryRequest,
}

impl Request for GetUserRequest {
    fn fields(self) -> Vec<RequestField> {
        if let Some(name) = self.name {
            return vec![RequestField {
                name: "name",
                value: Value::Text(name),
            }];
        }
        self.query.fields()
    }

    fn complete(&mut self, fields: HashMap<String, String>) -> Result<()> {
        self.name = parse_from_map!(fields, "name");
        if self.name.is_some() {
            return Ok(());
        }

        self.query.complete(fields)?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct PatchUserRequest {
    pub name: String,
    pub password: Option<String>,
    pub admin: Option<bool>,
}

impl Request for PatchUserRequest {
    fn fields(self) -> Vec<RequestField> {
        let mut fields = vec![RequestField {
            name: "name",
            value: Value::Text(self.name.clone()),
        }];
        if let Some(password) = self.password {
            fields.push(RequestField {
                name: "password",
                value: Value::Text(password),
            });
        }
        if let Some(admin) = self.admin {
            fields.push(RequestField {
                name: "admin",
                value: Value::Bool(admin),
            });
        }
        fields
    }

    fn complete(&mut self, mut fields: HashMap<String, String>) -> Result<()> {
        self.name = fields.remove("name").unwrap_or_default();
        if self.name.is_empty() {
            bail!("name is required to patch user");
        }

        self.password = fields.remove("password");
        self.admin = parse_from_map!(fields, "admin");

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DeleteUserRequest {
    pub name: String,
}

impl Request for DeleteUserRequest {
    fn fields(self) -> Vec<RequestField> {
        vec![RequestField {
            name: "name",
            value: Value::Text(self.name.clone()),
        }]
    }

    fn complete(&mut self, mut fields: HashMap<String, String>) -> Result<()> {
        self.name = fields.remove("name").unwrap_or_default();
        if self.name.is_empty() {
            bail!("name is required to delete user");
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expire_after: u64,
}
