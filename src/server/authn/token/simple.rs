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
