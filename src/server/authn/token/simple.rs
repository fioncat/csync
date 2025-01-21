use anyhow::{bail, Result};

use super::{TokenGenerator, TokenResponse, TokenValidator};

/// A simple token implementation that provides basic token generation and validation.
///
/// # Security Warning
/// This is an insecure implementation that simply prefixes the username with "simple-token-".
/// It should only be used for testing purposes and never in production environments.
/// For production use, please use `JwtTokenGenerator` instead.
#[derive(Debug, Clone)]
pub struct SimpleToken;

impl SimpleToken {
    /// Creates a new instance of SimpleToken.
    ///
    /// # Example
    /// ```
    /// let token_handler = SimpleToken::new();
    /// // Use for testing only
    /// let token_response = token_handler.generate_token("test_user".to_string())?;
    /// ```
    pub fn new() -> Self {
        Self
    }
}

impl TokenGenerator for SimpleToken {
    fn generate_token(&self, user: String) -> Result<TokenResponse> {
        if user.is_empty() {
            bail!("generate simple token failed: empty user");
        }
        Ok(TokenResponse {
            user: user.clone(),
            token: format!("simple-token-{user}"),
            expire_in: 0,
        })
    }
}

impl TokenValidator for SimpleToken {
    fn validate_token(&self, token: &str) -> Result<String> {
        match token.strip_prefix("simple-token-") {
            Some(user) => Ok(user.to_string()),
            None => bail!("invalid simple token"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::server::authn::token::tests::run_token_tests;

    use super::*;

    #[test]
    fn test_simple() {
        let token = SimpleToken::new();
        run_token_tests(&token, &token);
    }
}
