use rusqlite::types::Value;
use serde::{Deserialize, Serialize};

/// Represents a request type and necessary parameters for a resource operation
#[derive(Debug)]
pub enum ResourceRequest {
    /// Push new binary resource data.
    /// First parameter is resource metadata, format depends on specific resource type.
    /// Second parameter is the actual resource data.
    PutBinary(Option<String>, Vec<u8>),

    /// Push new JSON resource data
    PutJson(String),

    /// Retrieve a resource.
    /// First parameter is the resource ID.
    /// Second parameter indicates whether to only fetch resource metadata.
    /// If true, server will not return the actual resource data.
    Get(String, bool),

    /// List resources.
    /// First parameter specifies query conditions.
    /// Second parameter indicates whether to only fetch resource metadata.
    List(Query, bool),

    /// Delete a resource by its ID
    Delete(String),
}

/// Query conditions for listing resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// Query offset for pagination
    pub offset: Option<u64>,

    /// Maximum number of items to return
    pub limit: Option<u64>,

    /// Fuzzy search condition. The specific field to search depends on resource type
    pub search: Option<String>,

    /// Filter resources created after this timestamp
    pub since: Option<u64>,

    /// Filter resources created before this timestamp
    pub until: Option<u64>,

    /// Filter resources by owner
    pub owner: Option<String>,

    /// Filter resources by hash value
    pub hash: Option<String>,
}

/// Represents request or response payload body. The specific type is determined by headers.
/// When Content-Type is application/json, it represents JSON data.
/// Otherwise it's binary data, with metadata retrieved from Metadata header.
#[derive(Debug)]
pub enum Payload {
    /// Represents unparsed JSON response
    Json(String),
    /// Represents binary response. Contains metadata and binary data respectively
    Binary(Option<String>, Vec<u8>),
    /// Represents empty response
    None,
}

impl ResourceRequest {
    /// Returns the verb associated with this request (for RBAC control)
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
    /// Creates a new Query with default values (all fields set to None)
    pub fn default() -> Self {
        Self {
            offset: None,
            limit: None,
            search: None,
            since: None,
            until: None,
            owner: None,
            hash: None,
        }
    }

    /// Creates a new Query to search by user and hash
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

    /// Generates SQL WHERE clause based on query conditions
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

    /// Generates SQL LIMIT clause based on pagination parameters
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

    /// Converts query conditions into SQL parameters
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query() {
        // Test default constructor
        let query = Query::default();
        assert!(query.offset.is_none());
        assert!(query.limit.is_none());
        assert!(query.search.is_none());
        assert!(query.since.is_none());
        assert!(query.until.is_none());
        assert!(query.owner.is_none());
        assert!(query.hash.is_none());

        // Test new_hash constructor
        let query = Query::new_hash("test_user", "test_hash");
        assert_eq!(query.owner, Some("test_user".to_string()));
        assert_eq!(query.hash, Some("test_hash".to_string()));
        assert!(query.offset.is_none());
        assert!(query.limit.is_none());
        assert!(query.search.is_none());
        assert!(query.since.is_none());
        assert!(query.until.is_none());

        // Test WHERE clause generation
        let mut query = Query::default();
        assert_eq!(query.generate_where("name", "create_time"), "");

        // Test single condition
        query = Query::default();
        query.search = Some("test".to_string());
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE name LIKE ? "
        );

        // Test single time condition
        query = Query::default();
        query.since = Some(1000);
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE create_time >= ? "
        );

        // Test owner condition
        query = Query::default();
        query.owner = Some("user".to_string());
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE owner = ? "
        );

        // Test hash condition
        query = Query::default();
        query.hash = Some("hash123".to_string());
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE hash = ? "
        );

        // Test multiple conditions
        query = Query::default();
        query.search = Some("test".to_string());
        query.since = Some(1000);
        query.owner = Some("user".to_string());
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE name LIKE ? AND create_time >= ? AND owner = ? "
        );

        // Test all conditions
        query = Query {
            search: Some("test".to_string()),
            since: Some(1000),
            until: Some(2000),
            owner: Some("user".to_string()),
            hash: Some("hash123".to_string()),
            offset: None,
            limit: None,
        };
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE name LIKE ? AND create_time >= ? AND create_time <= ? AND owner = ? AND hash = ? "
        );

        // Test time range conditions
        query = Query::default();
        query.since = Some(1000);
        query.until = Some(2000);
        assert_eq!(
            query.generate_where("name", "create_time"),
            "WHERE create_time >= ? AND create_time <= ? "
        );

        // Test LIMIT clause generation
        let mut query = Query::default();
        assert_eq!(query.generate_limit(), "");

        query.limit = Some(10);
        assert_eq!(query.generate_limit(), "LIMIT ?");

        query.offset = Some(5);
        assert_eq!(query.generate_limit(), "LIMIT ? OFFSET ?");

        // Test params generation
        let query = Query {
            offset: Some(5),
            limit: Some(10),
            search: Some("test".to_string()),
            since: Some(1000),
            until: Some(2000),
            owner: Some("user".to_string()),
            hash: Some("hash123".to_string()),
        };

        let params = query.params();
        assert_eq!(params.len(), 7);

        // Check search parameter
        if let Value::Text(search) = &params[0] {
            assert_eq!(search, "%test%");
        } else {
            panic!("Expected Text value for search");
        }

        // Check since parameter
        if let Value::Integer(since) = params[1] {
            assert_eq!(since, 1000);
        } else {
            panic!("Expected Integer value for since");
        }

        // Check until parameter
        if let Value::Integer(until) = params[2] {
            assert_eq!(until, 2000);
        } else {
            panic!("Expected Integer value for until");
        }

        // Check owner parameter
        if let Value::Text(owner) = &params[3] {
            assert_eq!(owner, "user");
        } else {
            panic!("Expected Text value for owner");
        }

        // Check hash parameter
        if let Value::Text(hash) = &params[4] {
            assert_eq!(hash, "hash123");
        } else {
            panic!("Expected Text value for hash");
        }

        // Check limit parameter
        if let Value::Integer(limit) = params[5] {
            assert_eq!(limit, 10);
        } else {
            panic!("Expected Integer value for limit");
        }

        // Check offset parameter
        if let Value::Integer(offset) = params[6] {
            assert_eq!(offset, 5);
        } else {
            panic!("Expected Integer value for offset");
        }
    }
}
