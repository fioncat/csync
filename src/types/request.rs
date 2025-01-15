use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ResourceRequest {
    PutBinary(Option<String>, Vec<u8>),
    PutJson(String),
    Get(String, bool),
    List(Query, bool),
    Delete(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub offset: Option<u64>,
    pub limit: Option<u64>,

    pub search: Option<String>,

    pub since: Option<u64>,
    pub until: Option<u64>,

    pub owner: Option<String>,

    pub hash: Option<String>,
}

#[derive(Debug)]
pub enum Payload {
    Json(String),
    Binary(Option<String>, Vec<u8>),
    None,
}

impl ResourceRequest {
    pub fn verb(&self) -> &'static str {
        match self {
            ResourceRequest::PutBinary(_, _) => "put",
            ResourceRequest::PutJson(_) => "put",
            ResourceRequest::Get(_, head) => {
                if *head {
                    "head"
                } else {
                    "get"
                }
            }
            ResourceRequest::List(_, head) => {
                if *head {
                    "head"
                } else {
                    "get"
                }
            }
            ResourceRequest::Delete(_) => "delete",
        }
    }
}

impl Query {
    pub fn new_hash(user: &str, hash: &str) -> Self {
        Self {
            offset: None,
            limit: None,
            search: None,
            since: None,
            until: None,
            owner: Some(user.to_string()),
            hash: Some(hash.to_string()),
        }
    }

    pub fn generate_where(&self, search: &str, time: &str) -> String {
        let mut where_clause = vec![];
        if self.search.is_some() {
            where_clause.push(format!("{search} LIKE ?"));
        }
        if self.since.is_some() {
            where_clause.push(format!("{time} >= ?"));
        }
        if self.until.is_some() {
            where_clause.push(format!("{time} <= ?"));
        }
        if self.owner.is_some() {
            where_clause.push("owner = ?".to_string());
        }
        if self.hash.is_some() {
            where_clause.push("hash = ?".to_string());
        }
        if where_clause.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {} ", where_clause.join(" AND "))
        }
    }

    pub fn generate_limit(&self) -> &'static str {
        if self.limit.is_some() {
            if self.offset.is_some() {
                "LIMIT ? OFFSET ?"
            } else {
                "LIMIT ?"
            }
        } else {
            ""
        }
    }

    pub fn params(self) -> Vec<Value> {
        let mut params = vec![];
        if let Some(search) = self.search {
            params.push(Value::Text(format!("%{}%", search)));
        }
        if let Some(since) = self.since {
            params.push(Value::Integer(since as i64));
        }
        if let Some(until) = self.until {
            params.push(Value::Integer(until as i64));
        }
        if let Some(owner) = self.owner {
            params.push(Value::Text(owner));
        }
        if let Some(hash) = self.hash {
            params.push(Value::Text(hash));
        }
        if let Some(limit) = self.limit {
            params.push(Value::Integer(limit as i64));
        }
        if let Some(offset) = self.offset {
            params.push(Value::Integer(offset as i64));
        }
        params
    }
}
