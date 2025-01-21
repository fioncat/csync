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
