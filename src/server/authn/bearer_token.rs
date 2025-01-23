use actix_web::HttpRequest;
use anyhow::{bail, Result};

use super::token::TokenValidator;
use super::{Authenticator, AuthnResponse, AuthnUserInfo};

/// Bearer token authenticator that validates HTTP Authorization header.
///
/// This authenticator implements the Bearer token authentication scheme as defined in RFC 6750.
/// It extracts and validates tokens from the Authorization header with the following process:
///
/// 1. Checks for "Authorization" header
///    - If missing, continues to next authenticator
///    - If present, expects format: "Bearer <token>"
///
/// 2. Validates the token format:
///    - Must start with "Bearer" (case-insensitive)
///    - Must contain a non-empty token string
///    - Invalid format results in authentication failure
///
/// 3. Validates token using the provided TokenValidator:
///    - If validation succeeds, creates a new user with the returned username
///    - If validation fails, returns Unauthenticated response
///    - Empty username from validator results in an error
///
/// # Type Parameters
/// * `T` - Token validator implementation that verifies token authenticity
pub struct BearerTokenAuthenticator<T: TokenValidator> {
    validator: T,
}

impl<T: TokenValidator> BearerTokenAuthenticator<T> {
    /// Creates a new bearer token authenticator with the specified token validator.
    ///
    /// # Arguments
    /// * `validator` - Implementation of TokenValidator that will be used to verify tokens
    ///
    /// # Example
    /// ```
    /// let validator = JwtTokenValidator::new(public_key)?;
    /// let auth = BearerTokenAuthenticator::new(validator);
    /// ```
    pub fn new(validator: T) -> Self {
        Self { validator }
    }
}

impl<T: TokenValidator + Sync + Send> Authenticator for BearerTokenAuthenticator<T> {
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        _user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        let auth = match req.headers().get("Authorization") {
            Some(auth) => match auth.to_str() {
                Ok(auth) => auth.trim().to_string(),
                Err(_) => return Ok(AuthnResponse::Continue),
            },
            None => return Ok(AuthnResponse::Continue),
        };

        if auth.is_empty() {
            return Ok(AuthnResponse::Continue);
        }

        let mut iter = auth.split_whitespace();
        let bearer = iter.next();
        if bearer.is_none() {
            return Ok(AuthnResponse::Unauthenticated);
        }
        if bearer.unwrap().to_lowercase() != "bearer" {
            return Ok(AuthnResponse::Unauthenticated);
        }

        let token = match iter.next() {
            Some(token) => token,
            None => return Ok(AuthnResponse::Unauthenticated),
        };
        if token.is_empty() {
            return Ok(AuthnResponse::Unauthenticated);
        }

        let user = match self.validator.validate_token(token) {
            Ok(user) => user,
            Err(_) => return Ok(AuthnResponse::Unauthenticated),
        };
        if user.is_empty() {
            bail!("empty user identifier in token");
        }

        Ok(AuthnResponse::Ok(AuthnUserInfo {
            name: user,
            is_admin: false,
            is_anonymous: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;

    use crate::server::authn::token::simple::SimpleToken;

    use super::*;

    #[test]
    fn test_bearer_token() {
        let validator = SimpleToken::new();
        let auth = BearerTokenAuthenticator::new(validator);

        // Test no Authorization header
        let req = TestRequest::default().to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Continue));

        // Test empty Authorization header
        let req = TestRequest::default()
            .insert_header(("Authorization", ""))
            .to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Continue));

        // Test invalid Authorization format
        let req = TestRequest::default()
            .insert_header(("Authorization", "Basic abc"))
            .to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Unauthenticated));

        // Test empty token
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer "))
            .to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Unauthenticated));

        // Test invalid token
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer invalid-token"))
            .to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Unauthenticated));

        // Test valid token
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer simple-token-alice"))
            .to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "alice");
                assert!(!user.is_admin);
                assert!(!user.is_anonymous);
            }
            _ => panic!("expected Ok response"),
        }

        // Test case insensitive "Bearer"
        let req = TestRequest::default()
            .insert_header(("Authorization", "BEARER simple-token-bob"))
            .to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "bob");
                assert!(!user.is_admin);
                assert!(!user.is_anonymous);
            }
            _ => panic!("expected Ok response"),
        }
    }
}
