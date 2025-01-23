pub mod config;
pub mod factory;
pub mod jwt;

#[cfg(test)]
pub mod simple;

use anyhow::Result;

use crate::types::token::TokenResponse;

/// Token generator interface for creating authentication tokens.
///
/// This trait defines the standard way to generate authentication tokens for users.
/// Implementations should ensure that generated tokens are secure and contain
/// necessary user information for validation.
pub trait TokenGenerator {
    /// Generates a new authentication token for the specified user.
    ///
    /// # Arguments
    /// * `user` - Username to generate token for
    ///
    /// # Returns
    /// Returns a `TokenResponse` containing:
    /// - The username
    /// - The generated token
    /// - Token expiration timestamp (if applicable)
    fn generate_token(&self, user: String) -> Result<TokenResponse>;
}

/// Token validator interface for verifying authentication tokens.
///
/// This trait defines the standard way to validate authentication tokens.
/// Implementations should verify the token's authenticity, expiration,
/// and extract the associated username.
pub trait TokenValidator {
    /// Validates a token and extracts the associated username.
    ///
    /// # Arguments
    /// * `token` - The token string to validate
    ///
    /// # Returns
    /// Returns the username associated with the token if validation succeeds.
    fn validate_token(&self, token: &str) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use crate::time::advance_mock_time;

    use super::*;

    pub fn run_token_tests<TG, TV>(generator: &TG, validator: &TV)
    where
        TG: TokenGenerator,
        TV: TokenValidator,
    {
        let users = ["Alice", "Bob", "Carol", "David", "admin", "Test"];
        for user in users.iter() {
            let token = generator.generate_token(user.to_string()).unwrap();
            let result = validator.validate_token(&token.token).unwrap();
            assert_eq!(result, user.to_string());
        }

        assert!(generator.generate_token(String::new()).is_err());
        assert!(validator.validate_token("").is_err());
    }

    pub fn run_token_expiry_tests<TG, TV>(generator: &TG, validator: &TV, expiry: u64)
    where
        TG: TokenGenerator,
        TV: TokenValidator,
    {
        let token = generator.generate_token("Alice".to_string()).unwrap();
        advance_mock_time(expiry + 1);
        assert!(validator.validate_token(&token.token).is_err());
    }
}
