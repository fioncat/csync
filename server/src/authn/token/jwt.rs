use anyhow::{bail, Result};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::now::current_timestamp;

use super::{TokenGenerator, TokenResponse, TokenValidator};

/// JWT issuer identifier
const ISSUER: &str = "fioncat.io/csync/jwt-tokenizer";

/// Claims represents public claim values (as specified in RFC 7519)
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub aud: Option<String>, // Optional. The intended recipient of the token
    pub exp: usize,          // Required. Token expiration time (timestamp)
    pub iat: usize,          // Optional. Time at which token was issued (timestamp)
    pub iss: String,         // Optional. Token issuer
    pub nbf: usize,          // Optional. Time before which token must not be accepted (timestamp)
    pub sub: String,         // Optional. Subject of the token (user identifier)
}

/// JSON Web Token generator for creating signed tokens.
/// For more details, see: https://en.wikipedia.org/wiki/JSON_Web_Token
pub struct JwtTokenGenerator {
    key: EncodingKey, // Private key for signing
    expiry: usize,    // Token expiration time in seconds
}

impl JwtTokenGenerator {
    /// Creates a new JWT token generator that signs tokens using an RSA private key.
    ///
    /// # Arguments
    /// * `private_key` - RSA private key in PEM format
    /// * `expiry` - Token expiration time in seconds
    ///
    /// # Example
    /// ```
    /// let private_key = include_bytes!("private_key.pem");
    /// let generator = JwtTokenGenerator::new(private_key, 3600)?; // 1 hour expiry
    /// let token = generator.generate_token("user123".to_string())?;
    /// ```
    pub fn new(private_key: &[u8], expiry: u64) -> Result<Self> {
        let key = match EncodingKey::from_rsa_pem(private_key) {
            Ok(key) => key,
            Err(e) => bail!("parse RSA private key for jwt token generation failed: {e}"),
        };
        Ok(Self {
            key,
            expiry: expiry as usize,
        })
    }
}

impl TokenGenerator for JwtTokenGenerator {
    fn generate_token(&self, user: String) -> Result<TokenResponse> {
        if user.is_empty() {
            bail!("generate jwt token failed: empty user");
        }

        let now = current_timestamp() as usize;

        let claims = Claims {
            // TODO: now we don't know how to use audience, left it empty now, we will use
            // it in the future.
            aud: None,
            exp: now + self.expiry,
            iat: now,
            iss: String::from(ISSUER),
            nbf: now,
            sub: user,
        };

        // Sign the claims using RS256 algorithm
        match encode(&Header::new(Algorithm::RS256), &claims, &self.key) {
            Ok(token) => Ok(TokenResponse {
                user: claims.sub,
                token,
                expire_in: claims.exp,
            }),
            Err(e) => bail!("generate jwt token failed: {e}"),
        }
    }
}

/// JSON Web Token validator for verifying and decoding tokens.
/// Validates token signature, expiration time, and other claims.
pub struct JwtTokenValidator {
    key: DecodingKey, // Public key for verification
}

impl JwtTokenValidator {
    /// Creates a new JWT token validator using an RSA public key.
    ///
    /// # Arguments
    /// * `public_key` - RSA public key in PEM format
    ///
    /// # Example
    /// ```
    /// let public_key = include_bytes!("public_key.pem");
    /// let validator = JwtTokenValidator::new(public_key)?;
    /// let user = validator.validate_token(token)?; // Returns username if token is valid
    /// ```
    pub fn new(public_key: &[u8]) -> Result<Self> {
        let key = match DecodingKey::from_rsa_pem(public_key) {
            Ok(key) => key,
            Err(e) => bail!("parse RSA public key for jwt token validation failed: {e}"),
        };
        Ok(Self { key })
    }
}

impl TokenValidator for JwtTokenValidator {
    fn validate_token(&self, token: &str) -> Result<String> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[ISSUER]); // Validate issuer
        validation.set_required_spec_claims(&["exp", "iat", "iss", "nbf", "sub"]);

        // TODO: If audience validation is needed, set: validation.set_audience(&[audience]);

        // Verify token signature and decode
        let claims = match decode::<Claims>(token, &self.key, &validation) {
            Ok(data) => data.claims,
            Err(e) => bail!("validate jwt token failed: {e}"),
        };

        // Verify subject is not empty
        if claims.sub.is_empty() {
            bail!("validate jwt token failed: empty subject");
        }

        let now = current_timestamp() as usize;
        if now >= claims.exp {
            bail!("validate jwt token failed: token expired");
        }

        if now < claims.nbf {
            bail!("validate jwt token failed: token not yet valid");
        }

        Ok(claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use crate::authn::token::tests::{run_token_expiry_tests, run_token_tests};

    use super::*;

    #[test]
    fn test_jwt() {
        let private_key = include_bytes!("testdata/private_key.pem");
        let public_key = include_bytes!("testdata/public_key.pem");

        let jwt_generator = JwtTokenGenerator::new(private_key, 36000).unwrap();
        let jwt_validator = JwtTokenValidator::new(public_key).unwrap();

        run_token_tests(&jwt_generator, &jwt_validator);

        let jwt_generator = JwtTokenGenerator::new(private_key, 100).unwrap();
        run_token_expiry_tests(&jwt_generator, &jwt_validator, 100);

        let invalid_key = "invalid key".as_bytes();
        assert!(JwtTokenGenerator::new(invalid_key, 36000).is_err());
        assert!(JwtTokenValidator::new(invalid_key).is_err());
    }
}
