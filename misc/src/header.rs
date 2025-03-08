use std::collections::HashMap;

/// HeaderMap wraps a HashMap<String, String> and provides case-insensitive key lookup
#[derive(Debug, Clone, Default)]
pub struct HeaderMap {
    inner: HashMap<String, String>,
}

impl HeaderMap {
    /// Creates a new empty HeaderMap
    pub fn new() -> Self {
        HeaderMap {
            inner: HashMap::new(),
        }
    }

    /// Inserts a key-value pair into the map
    ///
    /// If the map already contains an entry with this key (case-insensitive),
    /// the value is updated and the old value is returned.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        let key_str = key.into();

        // Check if we already have this key in any case variation
        if let Some(existing_key) = self.find_key_case_insensitive(&key_str) {
            return self.inner.insert(existing_key, value.into());
        }

        // Otherwise insert with the original case
        self.inner.insert(key_str, value.into())
    }

    /// Returns a reference to the value corresponding to the key (case-insensitive)
    pub fn get(&self, key: &str) -> Option<&String> {
        if let Some(existing_key) = self.find_key_case_insensitive(key) {
            return self.inner.get(&existing_key);
        }
        None
    }

    /// Removes a key from the map, returning the value if the key was present
    pub fn remove(&mut self, key: &str) -> Option<String> {
        if let Some(existing_key) = self.find_key_case_insensitive(key) {
            return self.inner.remove(&existing_key);
        }
        None
    }

    /// Returns true if the map contains a key (case-insensitive)
    pub fn contains_key(&self, key: &str) -> bool {
        self.find_key_case_insensitive(key).is_some()
    }

    /// Returns the number of elements in the map
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the map is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the keys and values
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.inner.iter()
    }

    /// Clears the map, removing all key-value pairs
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Helper function to find a key in the map, ignoring case
    fn find_key_case_insensitive(&self, key: &str) -> Option<String> {
        for existing_key in self.inner.keys() {
            if existing_key.eq_ignore_ascii_case(key) {
                return Some(existing_key.clone());
            }
        }
        None
    }
}

// Implement IntoIterator for HeaderMap
impl IntoIterator for HeaderMap {
    type Item = (String, String);
    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

// Implement From<HashMap<String, String>> for HeaderMap
impl From<HashMap<String, String>> for HeaderMap {
    fn from(map: HashMap<String, String>) -> Self {
        HeaderMap { inner: map }
    }
}

// Implement Into<HashMap<String, String>> for HeaderMap
impl From<HeaderMap> for HashMap<String, String> {
    fn from(header_map: HeaderMap) -> Self {
        header_map.inner
    }
}

// Implement Index for HeaderMap to allow using [] syntax
impl std::ops::Index<&str> for HeaderMap {
    type Output = String;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("no entry found for key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_empty() {
        let headers = HeaderMap::new();
        assert!(headers.is_empty());
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");

        assert_eq!(
            headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(headers.len(), 1);
        assert!(!headers.is_empty());
    }

    #[test]
    fn test_case_insensitive_get() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");

        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            headers.get("CONTENT-TYPE"),
            Some(&"application/json".to_string())
        );
        assert_eq!(
            headers.get("ConTent-TyPe"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_update_preserves_original_case() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");

        // Update using different case
        headers.insert("content-TYPE", "text/html");

        // Check that the value is updated
        assert_eq!(headers.get("Content-Type"), Some(&"text/html".to_string()));

        // Check that number of entries is still 1 (not 2)
        assert_eq!(headers.len(), 1);

        // Check that original key case is preserved in the internal map
        let inner_map: HashMap<String, String> = headers.into();
        assert!(inner_map.contains_key("Content-Type"));
        assert!(!inner_map.contains_key("content-TYPE"));
    }

    #[test]
    fn test_remove() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("Authorization", "Bearer token123");

        assert_eq!(headers.len(), 2);

        // Remove using different case
        let removed = headers.remove("content-TYPE");
        assert_eq!(removed, Some("application/json".to_string()));
        assert_eq!(headers.len(), 1);
        assert_eq!(headers.get("Content-Type"), None);
    }

    #[test]
    fn test_contains_key() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");

        assert!(headers.contains_key("Content-Type"));
        assert!(headers.contains_key("content-type"));
        assert!(headers.contains_key("CONTENT-TYPE"));
        assert!(!headers.contains_key("X-Custom-Header"));
    }

    #[test]
    fn test_clear() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("Authorization", "Bearer token123");

        assert_eq!(headers.len(), 2);

        headers.clear();
        assert!(headers.is_empty());
        assert_eq!(headers.len(), 0);
    }

    #[test]
    fn test_iter() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("Authorization", "Bearer token123");

        let mut pairs = Vec::new();
        for (key, value) in headers.iter() {
            pairs.push((key.clone(), value.clone()));
        }

        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&("Content-Type".to_string(), "application/json".to_string())));
        assert!(pairs.contains(&("Authorization".to_string(), "Bearer token123".to_string())));
    }

    #[test]
    fn test_into_iter() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("Authorization", "Bearer token123");

        let mut pairs = Vec::new();
        for (key, value) in headers {
            pairs.push((key, value));
        }

        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&("Content-Type".to_string(), "application/json".to_string())));
        assert!(pairs.contains(&("Authorization".to_string(), "Bearer token123".to_string())));
    }

    #[test]
    fn test_from_hashmap() {
        let mut map = HashMap::new();
        map.insert("Content-Type".to_string(), "application/json".to_string());
        map.insert("Authorization".to_string(), "Bearer token123".to_string());

        let headers: HeaderMap = map.into();

        assert_eq!(headers.len(), 2);
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_into_hashmap() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");
        headers.insert("Authorization", "Bearer token123");

        let map: HashMap<String, String> = headers.into();

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("Content-Type"),
            Some(&"application/json".to_string())
        );
        // Case-sensitive in the regular HashMap
        assert_eq!(map.get("content-type"), None);
    }

    #[test]
    fn test_index_operator() {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", "application/json");

        assert_eq!(headers["content-type"], "application/json");
        assert_eq!(headers["CONTENT-TYPE"], "application/json");
    }

    #[test]
    #[should_panic(expected = "no entry found for key")]
    fn test_index_operator_panic() {
        let headers = HeaderMap::new();
        let _ = headers["Content-Type"];
    }

    #[test]
    fn test_empty_key() {
        let mut headers = HeaderMap::new();
        headers.insert("", "empty key");

        assert_eq!(headers.get(""), Some(&"empty key".to_string()));
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn test_edge_cases() {
        let mut headers = HeaderMap::new();

        // Insert header with space in name
        headers.insert("X-Custom Header", "value");
        assert_eq!(headers.get("x-custom header"), Some(&"value".to_string()));

        // Unicode characters
        headers.insert("X-Emoji", "😊");
        assert_eq!(headers.get("x-emoji"), Some(&"😊".to_string()));

        // Very long header name
        let long_name = "X-".to_string() + &"A".repeat(1000);
        headers.insert(&long_name, "long");
        assert_eq!(
            headers.get(&long_name.to_lowercase()),
            Some(&"long".to_string())
        );
    }

    #[test]
    fn test_multiple_case_variations() {
        let mut headers = HeaderMap::new();

        // First insert
        headers.insert("Content-Type", "application/json");
        assert_eq!(headers.len(), 1);

        // Insert with different case - should update, not add
        headers.insert("content-type", "text/html");
        assert_eq!(headers.len(), 1);

        // Another variation
        headers.insert("CONTENT-TYPE", "application/xml");
        assert_eq!(headers.len(), 1);

        // Mixed case
        headers.insert("Content-LENGTH", "256");
        assert_eq!(headers.len(), 2);

        // Check all inserted values
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/xml".to_string())
        );
        assert_eq!(headers.get("content-length"), Some(&"256".to_string()));
    }
}
