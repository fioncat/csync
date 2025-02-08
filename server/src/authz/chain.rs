use anyhow::Result;

use super::union::UnionAuthorizer;
use super::{Authorizer, AuthzRequest, AuthzResponse};

/// A chain of authorizers that are executed in sequence
///
/// This authorizer implements the Chain of Responsibility pattern:
/// - Authorizers are tried in order until a definitive decision is made
/// - If an authorizer returns Continue, the next one in chain is tried
/// - If all authorizers return Continue, the final result is Continue
pub struct ChainAuthorizer {
    /// The ordered list of authorizers to try
    pub(super) authorizers: Vec<UnionAuthorizer>,
}

impl ChainAuthorizer {
    /// Creates a new instance of ChainAuthorizer
    ///
    /// # Arguments
    /// * `authorizers` - Vector of authorizers to be executed in sequence
    ///
    /// # Returns
    /// * A new ChainAuthorizer instance configured with the provided authorizers
    pub fn new(authorizers: Vec<UnionAuthorizer>) -> Self {
        Self { authorizers }
    }
}

impl Authorizer for ChainAuthorizer {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse> {
        for authorizer in self.authorizers.iter() {
            match authorizer.authorize_request(req)? {
                AuthzResponse::Ok => return Ok(AuthzResponse::Ok),
                AuthzResponse::Continue => continue,
                AuthzResponse::Unauthorized => return Ok(AuthzResponse::Unauthorized),
            }
        }

        Ok(AuthzResponse::Continue)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use csync_misc::types::user::RoleRule;

    use crate::authn::AuthnUserInfo;
    use crate::authz::admin::AdminAuthorizer;
    use crate::authz::anonymous::AnonymousAuthorizer;
    use crate::authz::rule::RuleAuthorizer;
    use crate::db::{Database, RoleRecord};

    use super::*;

    #[test]
    fn test_chain() {
        // Create database and setup roles
        let db = Arc::new(Database::new_test());
        db.with_transaction(|tx, _cache| {
            tx.create_role(&RoleRecord {
                name: "user_role".to_string(),
                rules: vec![RoleRule {
                    resources: vec!["user_resource".to_string()].into_iter().collect(),
                    verbs: vec!["read".to_string()].into_iter().collect(),
                }],
                create_time: 0,
                update_time: 0,
            })?;
            tx.create_user_role("regular_user", "user_role")?;
            Ok(())
        })
        .unwrap();

        // Create chain with all types of authorizers
        let chain = ChainAuthorizer::new(vec![
            UnionAuthorizer::Admin(AdminAuthorizer::new()),
            UnionAuthorizer::Rule(RuleAuthorizer::new(db)),
            UnionAuthorizer::Anonymous(AnonymousAuthorizer::new(vec![RoleRule {
                resources: vec!["public_resource".to_string()].into_iter().collect(),
                verbs: vec!["read".to_string()].into_iter().collect(),
            }])),
        ]);

        // Test admin access (should be authorized by AdminAuthorizer)
        let admin_req = AuthzRequest {
            resource: "any_resource".to_string(),
            verb: "any_action".to_string(),
            user: AuthnUserInfo {
                name: "admin".to_string(),
                is_admin: true,
                is_anonymous: false,
            },
        };
        let result = chain.authorize_request(&admin_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "Admin should be authorized"
        );

        // Test regular user access to allowed resource (should be authorized by RuleAuthorizer)
        let user_req = AuthzRequest {
            resource: "user_resource".to_string(),
            verb: "read".to_string(),
            user: AuthnUserInfo {
                name: "regular_user".to_string(),
                is_admin: false,
                is_anonymous: false,
            },
        };
        let result = chain.authorize_request(&user_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "User should be authorized for allowed resource"
        );

        // Test anonymous access to public resource (should be authorized by AnonymousAuthorizer)
        let anon_req = AuthzRequest {
            resource: "public_resource".to_string(),
            verb: "read".to_string(),
            user: AuthnUserInfo {
                name: "anonymous".to_string(),
                is_admin: false,
                is_anonymous: true,
            },
        };
        let result = chain.authorize_request(&anon_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "Anonymous should be authorized for public resource"
        );

        // Test unauthorized access (should be denied)
        let denied_req = AuthzRequest {
            resource: "restricted_resource".to_string(),
            verb: "write".to_string(),
            user: AuthnUserInfo {
                name: "regular_user".to_string(),
                is_admin: false,
                is_anonymous: false,
            },
        };
        let result = chain.authorize_request(&denied_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Unauthorized),
            "Access should be denied for unauthorized request"
        );
    }
}
