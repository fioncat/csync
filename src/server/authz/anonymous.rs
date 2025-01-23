use anyhow::Result;

use crate::types::user::RoleRule;

use super::rule::is_authorized;
use super::{Authorizer, AuthzRequest, AuthzResponse};

/// An authorizer that handles anonymous user access
///
/// This authorizer implements access control for anonymous users:
/// - If the user is not anonymous, defers to the next authorizer
/// - If the user is anonymous, checks against configured rules
/// - Denies access if no matching rules are found
pub struct AnonymousAuthorizer {
    /// List of role rules that define what anonymous users can access
    rules: Vec<RoleRule>,
}

impl AnonymousAuthorizer {
    /// Creates a new instance of AnonymousAuthorizer
    ///
    /// # Arguments
    /// * `rules` - Vector of role rules that define anonymous user permissions
    ///
    /// # Returns
    /// * A new AnonymousAuthorizer instance configured with the specified rules
    pub fn new(rules: Vec<RoleRule>) -> Self {
        Self { rules }
    }
}

impl Authorizer for AnonymousAuthorizer {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse> {
        if !req.user.is_anonymous {
            return Ok(AuthzResponse::Continue);
        }

        if is_authorized(&self.rules, &req.resource, &req.verb) {
            return Ok(AuthzResponse::Ok);
        }

        Ok(AuthzResponse::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::authn::AuthnUserInfo;

    #[test]
    fn test_anonymous() {
        // Test setup
        let rules = vec![RoleRule {
            resources: vec!["public_resource".to_string()].into_iter().collect(),
            verbs: vec!["read".to_string()].into_iter().collect(),
        }];
        let authorizer = AnonymousAuthorizer::new(rules);

        // Test case 1: Non-anonymous user should continue
        let non_anon_req = AuthzRequest {
            resource: "any_resource".to_string(),
            verb: "any_action".to_string(),
            user: AuthnUserInfo {
                name: "regular_user".to_string(),
                is_admin: false,
                is_anonymous: false,
            },
        };
        let result = authorizer.authorize_request(&non_anon_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Continue),
            "Non-anonymous user should continue"
        );

        // Test case 2: Anonymous user with permitted access
        let allowed_anon_req = AuthzRequest {
            resource: "public_resource".to_string(),
            verb: "read".to_string(),
            user: AuthnUserInfo {
                name: "anonymous".to_string(),
                is_admin: false,
                is_anonymous: true,
            },
        };
        let result = authorizer.authorize_request(&allowed_anon_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "Anonymous user should be authorized for permitted resource/verb"
        );

        // Test case 3: Anonymous user with denied access
        let denied_anon_req = AuthzRequest {
            resource: "private_resource".to_string(),
            verb: "write".to_string(),
            user: AuthnUserInfo {
                name: "anonymous".to_string(),
                is_admin: false,
                is_anonymous: true,
            },
        };
        let result = authorizer.authorize_request(&denied_anon_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Unauthorized),
            "Anonymous user should be unauthorized for non-permitted resource/verb"
        );

        // Test case 4: Verify Send + Sync implementation
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AnonymousAuthorizer>();
    }
}
