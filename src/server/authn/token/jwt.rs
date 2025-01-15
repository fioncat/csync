use anyhow::{bail, Result};
use chrono::Local;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

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

pub struct JwtTokenGenerator {
    key: EncodingKey, // Private key for signing
    expiry: usize,
}

impl JwtTokenGenerator {
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
        let now = Local::now().timestamp() as usize;

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

pub struct JwtTokenValidator {
    key: DecodingKey,
}

impl JwtTokenValidator {
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

        let now = Local::now().timestamp() as usize;
        if now >= claims.exp {
            bail!("validate jwt token failed: token expired");
        }

        if now < claims.nbf {
            bail!("validate jwt token failed: token not yet valid");
        }

        Ok(claims.sub)
    }
}
