pub mod blob;
pub mod metadata;
pub mod user;

use std::collections::HashMap;
use std::fmt::Display;
use std::vec;

use anyhow::{bail, Result};
use blob::Blob;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::header::HeaderMap;

pub const HEALTHZ_PATH: &str = "/v1/healthz";

pub const HEADER_AUTHORIZATION: &str = "Authorization";
pub const HEADER_CONTENT_TYPE: &str = "Content-Type";
pub const MIME_JSON: &str = "application/json";
pub const MIME_OCTET_STREAM: &str = "application/octet-stream";

#[macro_export]
macro_rules! parse_from_map {
    ($fields:expr,$field:expr) => {
        match $fields.get($field) {
            Some(id) => match id.parse() {
                Ok(id) => Some(id),
                Err(_) => bail!(format!("{} is invalid", $field)),
            },
            None => None,
        }
    };
}

pub struct RequestField {
    pub name: &'static str,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum Value {
    Text(String),
    Integer(u64),
    Bool(bool),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Text(text) => write!(f, "{text}"),
            Value::Integer(integer) => write!(f, "{integer}"),
            Value::Bool(boolean) => write!(f, "{boolean}"),
        }
    }
}

pub trait Request: Default {
    fn fields(self) -> Vec<RequestField> {
        vec![]
    }
    fn complete(&mut self, _fields: HashMap<String, String>) -> Result<()> {
        Ok(())
    }

    fn append_headers(&self, _headers: &mut HashMap<&str, String>) {}
    fn complete_headers(&mut self, _headers: HeaderMap) -> Result<()> {
        Ok(())
    }

    fn is_data(&self) -> bool {
        false
    }
    fn set_data(&mut self, _data: Vec<u8>) {}
    fn data(self) -> Vec<u8> {
        vec![]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyRequest;

impl Request for EmptyRequest {}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct QueryRequest {
    pub offset: Option<u64>,
    pub limit: Option<u64>,

    pub search: Option<String>,

    pub update_after: Option<u64>,
    pub update_before: Option<u64>,
}

const DEFAULT_LIMIT: u64 = 10;

impl Request for QueryRequest {
    fn fields(self) -> Vec<RequestField> {
        let mut fields = Vec::new();
        if let Some(offset) = self.offset {
            fields.push(RequestField {
                name: "offset",
                value: Value::Integer(offset),
            });
        }

        if let Some(limit) = self.limit {
            fields.push(RequestField {
                name: "limit",
                value: Value::Integer(limit),
            });
        }

        if let Some(search) = self.search {
            fields.push(RequestField {
                name: "search",
                value: Value::Text(search),
            });
        }

        if let Some(update_after) = self.update_after {
            fields.push(RequestField {
                name: "update_after",
                value: Value::Integer(update_after),
            });
        }

        if let Some(update_before) = self.update_before {
            fields.push(RequestField {
                name: "update_before",
                value: Value::Integer(update_before),
            });
        }

        fields
    }

    fn complete(&mut self, mut fields: HashMap<String, String>) -> Result<()> {
        self.offset = parse_from_map!(fields, "offset");
        self.limit = parse_from_map!(fields, "limit");
        if self.limit.is_none() {
            self.limit = Some(DEFAULT_LIMIT);
        }
        self.search = fields.remove("search");
        self.update_after = parse_from_map!(fields, "update_after");
        self.update_before = parse_from_map!(fields, "update_before");

        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct Response<T: Serialize + DeserializeOwned> {
    pub code: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    #[serde(skip)]
    pub blob: Option<Blob>,
}

pub const STATUS_OK: u32 = 200;
pub const STATUS_BAD_REQUEST: u32 = 400;
pub const STATUS_UNAUTHORIZED: u32 = 401;
pub const STATUS_FORBIDDEN: u32 = 403;
pub const STATUS_NOT_FOUND: u32 = 404;
pub const STATUS_INTERNAL_SERVER_ERROR: u32 = 500;

impl<T: Serialize + DeserializeOwned> Response<T> {
    pub fn ok() -> Self {
        Self {
            code: STATUS_OK,
            message: None,
            data: None,
            blob: None,
        }
    }

    pub fn with_blob(blob: Blob) -> Self {
        Self {
            code: STATUS_OK,
            message: None,
            data: None,
            blob: Some(blob),
        }
    }

    pub fn with_data(data: T) -> Self {
        Self {
            code: STATUS_OK,
            message: None,
            data: Some(data),
            blob: None,
        }
    }

    pub fn bad_request(message: impl ToString) -> Self {
        Self {
            code: STATUS_BAD_REQUEST,
            message: Some(message.to_string()),
            data: None,
            blob: None,
        }
    }

    pub fn unauthorized(message: impl ToString) -> Self {
        Self {
            code: STATUS_UNAUTHORIZED,
            message: Some(message.to_string()),
            data: None,
            blob: None,
        }
    }

    pub fn not_found(message: impl ToString) -> Self {
        Self {
            code: STATUS_NOT_FOUND,
            message: Some(message.to_string()),
            data: None,
            blob: None,
        }
    }

    pub fn resource_not_found() -> Self {
        Self::not_found("Resource not found")
    }

    pub fn internal_server_error(message: impl ToString) -> Self {
        Self {
            code: STATUS_INTERNAL_SERVER_ERROR,
            message: Some(message.to_string()),
            data: None,
            blob: None,
        }
    }

    pub fn forbidden() -> Self {
        Self {
            code: STATUS_FORBIDDEN,
            message: Some(String::from("Operation not allowed")),
            data: None,
            blob: None,
        }
    }

    pub fn database_error() -> Self {
        Self::internal_server_error("Database error")
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct ListResponse<T: Serialize + DeserializeOwned> {
    pub items: Vec<T>,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub version: String,
    pub timestamp: u64,
}
