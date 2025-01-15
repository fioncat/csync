pub mod config;
pub mod factory;
pub mod jwt;

#[cfg(test)]
mod simple;

use anyhow::Result;

use crate::types::token::TokenResponse;

pub trait TokenGenerator {
    fn generate_token(&self, user: String) -> Result<TokenResponse>;
}

pub trait TokenValidator {
    fn validate_token(&self, token: &str) -> Result<String>;
}
