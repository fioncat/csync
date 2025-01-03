use anyhow::{bail, Result};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::Tokenizer;

/// Claims represents public claim values (as specified in RFC 7519)
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    pub aud: Option<String>, // Optional. The intended recipient of the token
    pub exp: usize,          // Required. Token expiration time (UTC timestamp)
    pub iat: usize,          // Optional. Time at which token was issued (UTC timestamp)
    pub iss: String,         // Optional. Token issuer
    pub nbf: usize, // Optional. Time before which token must not be accepted (UTC timestamp)
    pub sub: String, // Optional. Subject of the token (user identifier)
}

/// Implementation for JWT token generation and validation
pub(super) struct JwtTokenizer {
    encoding_key: EncodingKey, // Private key for signing
    decoding_key: DecodingKey, // Public key for verification
}

impl JwtTokenizer {
    /// JWT issuer identifier
    const ISSUER: &'static str = "fioncat.io/csync/jwt-tokenizer";

    /// JSON Web Token (JWT) is a compact, URL-safe means of representing claims
    /// between parties:
    ///
    /// - Self-contained: carries all necessary information about the user
    /// - Stateless: no need to query the database for validation
    /// - Secure: digitally signed using RSA cryptography
    /// - Compact: can be sent through URL, POST parameter, or HTTP header
    ///
    /// See:
    ///
    /// - https://tools.ietf.org/html/rfc7519
    /// - https://en.wikipedia.org/wiki/JSON_Web_Token
    ///
    /// # Arguments
    /// * `private_key` - RSA private key for signing
    /// * `public_key` - RSA public key for verification
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::server::authn::rsa::generate_rsa_keys;
    /// use crate::server::authn::token::Tokenizer;
    /// use crate::server::authn::token::jwt::JwtTokenizer;
    ///
    /// // Generate RSA keys (in PEM format)
    /// let (public_key, private_key) = generate_rsa_keys().unwrap();
    ///
    /// // Create a new tokenizer
    /// let tokenizer = JwtTokenizer::new(private_key, public_key).unwrap();
    ///
    /// // Generate a token that expires in 1 hour (3600 seconds)
    /// let token = tokenizer.generate_token("user123".to_string(), 3600).unwrap();
    ///
    /// // Validate the token and get the user ID
    /// let user = tokenizer.validate_token(&token).unwrap();
    /// assert_eq!(user, "user123");
    /// ```
    pub fn new(private_key: &[u8], public_key: &[u8]) -> Result<Self> {
        let encoding_key = match EncodingKey::from_rsa_pem(private_key) {
            Ok(key) => key,
            Err(e) => bail!("parse RSA private key for jwt token generation failed: {e}"),
        };
        let decoding_key = match DecodingKey::from_rsa_pem(public_key) {
            Ok(key) => key,
            Err(e) => bail!("parse RSA public key for jwt token generation failed: {e}"),
        };
        Ok(Self {
            encoding_key,
            decoding_key,
        })
    }
}

impl Tokenizer for JwtTokenizer {
    /// Generates a JWT token
    fn generate_token(&self, user: String, expiry: usize) -> Result<String> {
        if expiry == 0 {
            bail!("expiry must be greater than 0");
        }
        let now = Utc::now().timestamp() as usize;

        let claims = Claims {
            // TODO: now we don't know how to use audience, left it empty now, we will use
            // it in the future.
            aud: None,
            exp: now + expiry,
            iat: now,
            iss: String::from(Self::ISSUER),
            nbf: now,
            sub: user,
        };

        // Sign the claims using RS256 algorithm
        match encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key) {
            Ok(token) => Ok(token),
            Err(e) => bail!("generate jwt token failed: {e}"),
        }
    }

    /// Validates a JWT token and returns the user identifier
    fn validate_token(&self, token: &str) -> Result<String> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[Self::ISSUER]); // Validate issuer
        validation.set_required_spec_claims(&["exp", "iat", "iss", "nbf", "sub"]);

        // TODO: If audience validation is needed, set: validation.set_audience(&[audience]);

        // Verify token signature and decode
        let claims = match decode::<Claims>(token, &self.decoding_key, &validation) {
            Ok(data) => data.claims,
            Err(e) => bail!("validate jwt token failed: {e}"),
        };

        // Verify subject is not empty
        if claims.sub.is_empty() {
            bail!("validate jwt token failed: empty subject");
        }

        let now = Utc::now().timestamp() as usize;
        if now >= claims.exp {
            bail!("validate jwt token failed: token expired");
        }

        if now < claims.nbf {
            bail!("validate jwt token failed: token not yet valid");
        }

        Ok(claims.sub)
    }
}

/// Unit tests module
#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;

    use crate::server::authn::rsa::generate_rsa_keys;

    use super::*;

    /// Tests token generation and validation
    #[test]
    fn test_generate_and_validate() {
        // Prepare test key pairs
        let (generated_pubkey, generated_privkey) = generate_rsa_keys().unwrap();
        let read_pubkey = include_bytes!("testdata/public_key.pem");
        let read_privkey = include_bytes!("testdata/private_key.pem");

        let wrong_pubkey = include_bytes!("testdata/wrong_public_key.pem");
        let wrong_privkey = include_bytes!("testdata/wrong_private_key.pem");

        // Define test case structure
        struct TestCase {
            public_key: Vec<u8>,
            private_key: Vec<u8>,
            user: String,
            expect_ok: bool, // Whether the test is expected to succeed
            expired: bool,   // Whether to test expiration scenario
        }

        // Test cases list
        let test_cases = vec![
            // Test with dynamically generated key pair
            TestCase {
                public_key: generated_pubkey.clone(),
                private_key: generated_privkey.clone(),
                user: String::from("test_user_generated"),
                expect_ok: true,
                expired: false,
            },
            // Test token expiration scenario
            TestCase {
                public_key: generated_pubkey.clone(),
                private_key: generated_privkey.clone(),
                user: String::from("test_user_generated_expire"),
                expect_ok: false,
                expired: true,
            },
            // Test with predefined key pair
            TestCase {
                public_key: read_pubkey.to_vec(),
                private_key: read_privkey.to_vec(),
                user: String::from("test_user_read"),
                expect_ok: true,
                expired: false,
            },
            // Test expiration with predefined keys
            TestCase {
                public_key: read_pubkey.to_vec(),
                private_key: read_privkey.to_vec(),
                user: String::from("test_user_read_expire"),
                expect_ok: false,
                expired: true,
            },
            // Test with wrong key pair (validation should fail)
            TestCase {
                public_key: wrong_pubkey.to_vec(),
                private_key: wrong_privkey.to_vec(),
                user: String::from("test_user_wrong_key"),
                expect_ok: false,
                expired: false,
            },
        ];

        // Execute test cases
        for test_case in test_cases {
            println!("test: {}", test_case.user);
            let result = || -> Result<()> {
                let tokenizer = JwtTokenizer::new(&test_case.private_key, &test_case.public_key)?;
                let token = tokenizer.generate_token(test_case.user.clone(), 1)?;
                if test_case.expired {
                    sleep(Duration::from_secs(2)); // Wait for token to expire
                }
                let user = tokenizer.validate_token(&token)?;
                assert_eq!(user, test_case.user);
                Ok(())
            }();
            if test_case.expect_ok {
                result.unwrap();
            } else {
                assert!(result.is_err());
            }
        }
    }
}
