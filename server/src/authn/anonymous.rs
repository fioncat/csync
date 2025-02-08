use actix_web::HttpRequest;
use anyhow::Result;

use super::{Authenticator, AuthnResponse, AuthnUserInfo};

/// Authenticator that provides anonymous access when no other authentication is available.
///
/// This authenticator acts as a fallback mechanism in the authentication chain.
/// If a user is already authenticated, it preserves their identity.
/// Otherwise, it creates an anonymous user with limited privileges.
pub struct AnonymousAuthenticator;

impl AnonymousAuthenticator {
    /// Creates a new AnonymousAuthenticator.
    pub fn new() -> Self {
        Self {}
    }
}

impl Authenticator for AnonymousAuthenticator {
    fn authenticate_request(
        &self,
        _req: &HttpRequest,
        user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        if let Some(user) = user {
            return Ok(AuthnResponse::Ok(user));
        }

        Ok(AuthnResponse::Ok(AuthnUserInfo {
            name: String::new(),
            is_admin: false,
            is_anonymous: true,
        }))
    }
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;
    use super::*;

    fn mock_user(name: &str) -> AuthnUserInfo {
        AuthnUserInfo {
            name: name.to_string(),
            is_admin: false,
            is_anonymous: false,
        }
    }

    #[test]
    fn test_anonymous() {
        let auth = AnonymousAuthenticator::new();
        let req = TestRequest::default().to_http_request();

        // Test with no user info - should create anonymous user
        let resp = auth.authenticate_request(&req, None).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert!(user.name.is_empty());
                assert!(!user.is_admin);
                assert!(user.is_anonymous);
            }
            _ => panic!("expected Ok response with anonymous user"),
        }

        // Test with existing user - should preserve user info
        let user = mock_user("alice");
        let resp = auth.authenticate_request(&req, Some(user)).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "alice");
                assert!(!user.is_admin);
                assert!(!user.is_anonymous);
            }
            _ => panic!("expected Ok response with existing user"),
        }

        // Test with admin user - should preserve admin status
        let mut admin = mock_user("admin");
        admin.is_admin = true;
        let resp = auth.authenticate_request(&req, Some(admin)).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "admin");
                assert!(user.is_admin);
                assert!(!user.is_anonymous);
            }
            _ => panic!("expected Ok response with admin user"),
        }
    }
}
