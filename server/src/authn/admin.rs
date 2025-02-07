use std::collections::HashSet;

use actix_web::HttpRequest;
use anyhow::Result;

use super::{Authenticator, AuthnResponse, AuthnUserInfo};

/// Authenticator for handling admin user privileges.
///
/// This authenticator checks if the user is 'admin' and validates their access based on
/// a configured IP allow list. For non-admin users, it simply passes through the authentication.
///
/// # Admin Access Control
/// - If allow_list contains "*", admin access is granted from any IP
/// - Otherwise, admin access is only granted if the client's IP is in the allow_list
/// - For security reasons, admin access is denied if client IP cannot be determined
pub struct AdminAuthenticator {
    allow_list: HashSet<String>,
}

impl AdminAuthenticator {
    /// Creates a new admin authenticator with the specified IP allow list.
    ///
    /// # Arguments
    /// * `allow_list` - Set of allowed IP addresses. Use "*" to allow all IPs.
    pub fn new(allow_list: HashSet<String>) -> Self {
        Self { allow_list }
    }
}

impl Authenticator for AdminAuthenticator {
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        if user.is_none() {
            return Ok(AuthnResponse::Continue);
        }

        let mut user = user.unwrap();
        if user.name != "admin" {
            return Ok(AuthnResponse::Ok(user));
        }

        if self.allow_list.contains("*") {
            user.is_admin = true;
            return Ok(AuthnResponse::Ok(user));
        }

        let conn_info = req.connection_info();
        let addr = match conn_info.peer_addr() {
            Some(addr) => addr,
            None => return Ok(AuthnResponse::Unauthenticated),
        };
        if !self.allow_list.contains(addr) {
            return Ok(AuthnResponse::Unauthenticated);
        }

        user.is_admin = true;
        Ok(AuthnResponse::Ok(user))
    }
}

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;
    use std::collections::HashSet;

    use super::*;

    fn mock_user(name: &str) -> AuthnUserInfo {
        AuthnUserInfo {
            name: name.to_string(),
            is_admin: false,
            is_anonymous: false,
        }
    }

    #[test]
    fn test_admin() {
        // Test no user info case
        let auth = AdminAuthenticator::new(HashSet::new());
        let req = TestRequest::default().to_http_request();
        let resp = auth.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Continue));

        // Test non-admin user
        let user = mock_user("alice");
        let resp = auth.authenticate_request(&req, Some(user)).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "alice");
                assert!(!user.is_admin);
            }
            _ => panic!("expected Ok response"),
        }

        // Test admin user with wildcard allow list
        let mut allow_list = HashSet::new();
        allow_list.insert("*".to_string());
        let auth = AdminAuthenticator::new(allow_list);

        let user = mock_user("admin");
        let resp = auth.authenticate_request(&req, Some(user)).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "admin");
                assert!(user.is_admin);
            }
            _ => panic!("expected Ok response"),
        }

        // Test admin user with specific IP allow list
        let mut allow_list = HashSet::new();
        allow_list.insert("127.0.0.1".to_string());
        let auth = AdminAuthenticator::new(allow_list);

        // Test allowed IP
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:1234".parse().unwrap())
            .to_http_request();
        let user = mock_user("admin");
        let resp = auth.authenticate_request(&req, Some(user)).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "admin");
                assert!(user.is_admin);
            }
            _ => panic!("expected Ok response"),
        }

        // Test disallowed IP
        let req = TestRequest::default()
            .peer_addr("192.168.1.1:1234".parse().unwrap())
            .to_http_request();
        let user = mock_user("admin");
        let resp = auth.authenticate_request(&req, Some(user)).unwrap();
        assert!(matches!(resp, AuthnResponse::Unauthenticated));

        // Test missing peer address
        let req = TestRequest::default().to_http_request();
        let user = mock_user("admin");
        let resp = auth.authenticate_request(&req, Some(user)).unwrap();
        assert!(matches!(resp, AuthnResponse::Unauthenticated));
    }
}
