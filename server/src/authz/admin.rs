use anyhow::Result;

use super::{Authorizer, AuthzRequest, AuthzResponse};

/// An authorizer that grants access to admin users
///
/// This authorizer implements a simple admin check:
/// - If the user is an admin, access is granted
/// - If the user is not an admin, defers to the next authorizer
pub struct AdminAuthorizer;

impl AdminAuthorizer {
    /// Creates a new instance of AdminAuthorizer
    ///
    /// # Returns
    /// * A new AdminAuthorizer instance
    pub fn new() -> Self {
        Self
    }
}

impl Authorizer for AdminAuthorizer {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse> {
        if req.user.is_admin {
            return Ok(AuthzResponse::Ok);
        }

        Ok(AuthzResponse::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authn::AuthnUserInfo;

    #[test]
    fn test_admin() {
        // Test setup
        let authorizer = AdminAuthorizer::new();

        // Test case 1: Admin user should be authorized
        let admin_req = AuthzRequest {
            resource: "any_resource".to_string(),
            verb: "any_action".to_string(),
            user: AuthnUserInfo {
                name: "admin_user".to_string(),
                is_admin: true,
                is_anonymous: false,
            },
        };
        let result = authorizer.authorize_request(&admin_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "Admin user should be authorized"
        );

        // Test case 2: Non-admin user should continue
        let non_admin_req = AuthzRequest {
            resource: "any_resource".to_string(),
            verb: "any_action".to_string(),
            user: AuthnUserInfo {
                name: "normal_user".to_string(),
                is_admin: false,
                is_anonymous: false,
            },
        };
        let result = authorizer.authorize_request(&non_admin_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Continue),
            "Non-admin user should continue"
        );

        // Test case 3: Verify Send + Sync implementation
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AdminAuthorizer>();
    }
}
