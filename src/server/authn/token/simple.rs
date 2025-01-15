use anyhow::{bail, Result};

use super::{TokenGenerator, TokenResponse, TokenValidator};

#[derive(Debug, Clone)]
pub struct SimpleToken;

impl SimpleToken {
    pub fn new() -> Self {
        Self
    }
}

impl TokenGenerator for SimpleToken {
    fn generate_token(&self, user: String) -> Result<TokenResponse> {
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
