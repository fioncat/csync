use std::sync::Arc;

use anyhow::Result;

use crate::server::db::Database;
use crate::types::user::RoleRule;

use super::{Authorizer, AuthzRequest, AuthzResponse};

/// An authorizer that handles role-based access control
///
/// This authorizer implements access control based on user roles:
/// - Skips admin and anonymous users (returns Continue)
/// - Loads user roles from database (with caching)
/// - Checks if any role rules allow the requested access
pub struct RuleAuthorizer {
    /// Database connection for loading user roles
    db: Arc<Database>,
}

impl RuleAuthorizer {
    /// Creates a new instance of RuleAuthorizer
    ///
    /// # Arguments
    /// * `db` - Arc wrapped database connection
    ///
    /// # Returns
    /// * A new RuleAuthorizer instance configured with the database
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl Authorizer for RuleAuthorizer {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse> {
        if req.user.is_admin || req.user.is_anonymous {
            return Ok(AuthzResponse::Continue);
        }

        let roles = self.db.with_transaction(|tx, cache| {
            if let Some(roles) = cache.list_user_roles(&req.user.name)? {
                return Ok(roles);
            }

            let roles = tx.list_user_roles(&req.user.name)?;
            cache.save_user_roles(&req.user.name, roles.clone())?;

            Ok(roles)
        })?;
        for role in roles {
            if is_authorized(&role.rules, &req.resource, &req.verb) {
                return Ok(AuthzResponse::Ok);
            }
        }

        Ok(AuthzResponse::Unauthorized)
    }
}

/// Checks if any rule in the provided rules allows access to the resource and verb
///
/// # Arguments
/// * `rules` - Slice of role rules to check
/// * `resource` - The resource being accessed
/// * `verb` - The action being performed
///
/// # Returns
/// * `true` if access is allowed, `false` otherwise
pub fn is_authorized(rules: &[RoleRule], resource: &str, verb: &str) -> bool {
    for rule in rules.iter() {
        if !rule.resources.contains("*") && !rule.resources.contains(resource) {
            continue;
        }
        if !rule.verbs.contains("*") && !rule.verbs.contains(verb) {
            continue;
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::server::{authn::AuthnUserInfo, db::RoleRecord};

    use super::*;

    #[test]
    fn test_rule_authorizer() {
        // Test setup
        let db = Arc::new(Database::new_test());
        let authorizer = RuleAuthorizer::new(db.clone());

        db.with_transaction(|tx, _cache| {
            tx.create_role(&RoleRecord {
                name: "wheel".to_string(),
                rules: vec![
                    RoleRule {
                        resources: vec!["allowed_resource".to_string()].into_iter().collect(),
                        verbs: vec!["*".to_string()].into_iter().collect(),
                    },
                    RoleRule {
                        resources: vec!["restricted_resource".to_string()]
                            .into_iter()
                            .collect(),
                        verbs: vec!["read".to_string()].into_iter().collect(),
                    },
                ],
                create_time: 0,
                update_time: 0,
            })
            .unwrap();
            tx.create_user_role("regular_user", "wheel").unwrap();
            Ok(())
        })
        .unwrap();

        // Test case 1: Admin user should continue
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
            matches!(result, AuthzResponse::Continue),
            "Admin user should continue"
        );

        // Test case 2: Anonymous user should continue
        let anon_req = AuthzRequest {
            resource: "any_resource".to_string(),
            verb: "any_action".to_string(),
            user: AuthnUserInfo {
                name: "anonymous".to_string(),
                is_admin: false,
                is_anonymous: true,
            },
        };
        let result = authorizer.authorize_request(&anon_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Continue),
            "Anonymous user should continue"
        );

        // Test case 3: Regular user with permitted access
        let regular_req = AuthzRequest {
            resource: "allowed_resource".to_string(),
            verb: "read".to_string(),
            user: AuthnUserInfo {
                name: "regular_user".to_string(),
                is_admin: false,
                is_anonymous: false,
            },
        };

        // First request - should hit database
        let result = authorizer.authorize_request(&regular_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "Regular user should be authorized for permitted resource/verb"
        );

        // Second request - should hit cache
        let result = authorizer.authorize_request(&regular_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Ok),
            "Cached result should also authorize"
        );

        // TODO: Verify cache was hit

        // Test case 4: Regular user with denied access
        let denied_req = AuthzRequest {
            resource: "restricted_resource".to_string(),
            verb: "write".to_string(),
            user: AuthnUserInfo {
                name: "regular_user".to_string(),
                is_admin: false,
                is_anonymous: false,
            },
        };
        let result = authorizer.authorize_request(&denied_req).unwrap();
        assert!(
            matches!(result, AuthzResponse::Unauthorized),
            "Regular user should be unauthorized for restricted resource/verb"
        );
    }

    #[test]
    fn test_is_authorized() {
        let rules = vec![
            RoleRule {
                resources: vec!["resource1".to_string()].into_iter().collect(),
                verbs: vec!["read".to_string()].into_iter().collect(),
            },
            RoleRule {
                resources: vec!["resource2".to_string()].into_iter().collect(),
                verbs: vec!["*".to_string()].into_iter().collect(),
            },
        ];

        // Test wildcard verb
        assert!(is_authorized(&rules, "resource2", "any_verb"));

        // Test specific resource and verb
        assert!(is_authorized(&rules, "resource1", "read"));

        // Test denied access
        assert!(!is_authorized(&rules, "resource1", "write"));
        assert!(!is_authorized(&rules, "resource3", "read"));
    }
}
