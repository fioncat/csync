use anyhow::{bail, Result};
use csync_misc::api::user::{TokenResponse, User};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT issuer identifier
const ISSUER: &str = "fioncat.io/csync/jwt-tokenizer";

/// Claims represents public claim values (as specified in RFC 7519)
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub aud: String, // Optional. The intended recipient of the token
    pub exp: usize,  // Required. Token expiration time (timestamp)
    pub iat: usize,  // Optional. Time at which token was issued (timestamp)
    pub iss: String, // Optional. Token issuer
    pub nbf: usize,  // Optional. Time before which token must not be accepted (timestamp)
    pub sub: String, // Optional. Subject of the token (user identifier)
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

    #[cfg(test)]
    pub fn new_test() -> Self {
        let private_key = include_bytes!("testdata/private_key.pem");
        Self::new(private_key, 60).unwrap()
    }

    pub fn generate_token(&self, user: User, now: u64) -> Result<TokenResponse> {
        let now = now as usize;

        let aud = if user.admin { "admin" } else { "normal" };

        let claims = Claims {
            aud: String::from(aud),
            exp: now + self.expiry,
            iat: now,
            iss: String::from(ISSUER),
            nbf: now,
            sub: user.name,
        };

        // Sign the claims using RS256 algorithm
        match encode(&Header::new(Algorithm::RS256), &claims, &self.key) {
            Ok(token) => Ok(TokenResponse {
                token,
                expire_after: claims.exp as u64,
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
    pub fn new(public_key: &[u8]) -> Result<Self> {
        let key = match DecodingKey::from_rsa_pem(public_key) {
            Ok(key) => key,
            Err(e) => bail!("parse RSA public key for jwt token validation failed: {e}"),
        };
        Ok(Self { key })
    }

    #[cfg(test)]
    pub fn new_test() -> Self {
        let public_key = include_bytes!("testdata/public_key.pem");
        Self::new(public_key).unwrap()
    }

    pub fn validate_token(&self, token: &str, now: u64) -> Result<User> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[ISSUER]); // Validate issuer
        validation.set_required_spec_claims(&["aud", "exp", "iat", "iss", "nbf", "sub"]);
        validation.set_audience(&["admin", "normal"]);

        // Verify token signature and decode
        let claims = match decode::<Claims>(token, &self.key, &validation) {
            Ok(data) => data.claims,
            Err(e) => bail!("validate jwt token failed: {e}"),
        };

        // Verify subject is not empty
        if claims.sub.is_empty() {
            bail!("validate jwt token failed: empty subject");
        }

        let now = now as usize;
        if now >= claims.exp {
            bail!("validate jwt token failed: token expired");
        }

        if now < claims.nbf {
            bail!("validate jwt token failed: token not yet valid");
        }

        let admin = claims.aud == "admin";

        Ok(User {
            name: claims.sub,
            admin,
            update_time: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_jwt() {
        let jwt_generator = JwtTokenGenerator::new_test();
        let jwt_validator = JwtTokenValidator::new_test();

        let users = [
            User {
                name: String::from("alice"),
                admin: true,
                update_time: 0,
            },
            User {
                name: String::from("Bob"),
                admin: false,
                update_time: 0,
            },
            User {
                name: String::from("test"),
                admin: true,
                update_time: 0,
            },
        ];

        let now = Utc::now().timestamp() as u64;
        for user in users {
            let token = jwt_generator.generate_token(user.clone(), now).unwrap();
            let result = jwt_validator.validate_token(&token.token, now).unwrap();
            assert_eq!(result, user);

            let result = jwt_validator.validate_token(&token.token, now + 80);
            assert!(result.is_err());
        }
    }
}
