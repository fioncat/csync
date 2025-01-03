mod jwt;

pub mod config;
pub mod factory;

#[cfg(test)]
pub mod simple;

use anyhow::Result;

/// Trait for token generation and validation
///
/// This trait defines the interface for token-based authentication systems.
/// Implementations can use different token formats and algorithms (e.g., JWT, simple tokens).
pub trait Tokenizer {
    /// Generates a new authentication token for the given user
    ///
    /// # Arguments
    /// * `user` - The user identifier to embed in the token
    /// * `expiry` - Token expiration time in seconds
    ///
    /// # Returns
    /// * `Result<String>` - The generated token string on success, or an error
    fn generate_token(&self, user: String, expiry: usize) -> Result<String>;

    /// Validates a token and extracts the user identifier
    ///
    /// # Arguments
    /// * `token` - The token string to validate
    ///
    /// # Returns
    /// * `Result<String>` - The user identifier on successful validation, or an error
    fn validate_token(&self, token: &str) -> Result<String>;
}
