use anyhow::{bail, Result};

use super::Tokenizer;

/// SimpleTokenizer is a basic implementation of the Tokenizer trait intended for testing purposes only.
/// WARNING: This implementation is NOT secure and should never be used in production environments.
/// It simply concatenates a prefix with the username without any encryption or validation.
pub(super) struct SimpleTokenizer;

impl SimpleTokenizer {
    /// Creates a new SimpleTokenizer instance
    pub fn new() -> Self {
        Self
    }
}

impl Tokenizer for SimpleTokenizer {
    /// Generates an insecure token by concatenating a prefix with the username.
    /// The expiry parameter is ignored.
    fn generate_token(&self, user: String, _expiry: usize) -> Result<String> {
        Ok(format!("simple-token-{user}"))
    }

    /// Validates a token by checking for the expected prefix and extracting the username.
    /// No cryptographic validation is performed.
    fn validate_token(&self, token: &str) -> Result<String> {
        match token.strip_prefix("simple-token-") {
            Some(user) => Ok(user.to_string()),
            None => bail!("invalid simple token"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate() {
        let tokenizer = SimpleTokenizer::new();

        // Test valid token generation and validation
        let user = "test_user".to_string();
        let token = tokenizer.generate_token(user.clone(), 3600).unwrap();
        let validated_user = tokenizer.validate_token(&token).unwrap();
        assert_eq!(validated_user, user);

        // Test invalid token format
        let invalid_token = "invalid-token-format";
        assert!(tokenizer.validate_token(invalid_token).is_err());

        // Test wrong prefix
        let wrong_prefix = "wrong-prefix-test_user";
        assert!(tokenizer.validate_token(wrong_prefix).is_err());
    }
}
