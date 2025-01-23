use log::info;
use std::sync::Arc;

use crate::server::authz::anonymous::AnonymousAuthorizer;
use crate::server::db::Database;

use super::admin::AdminAuthorizer;
use super::chain::ChainAuthorizer;
use super::config::AuthzConfig;
use super::rule::RuleAuthorizer;
use super::union::UnionAuthorizer;

/// Factory for building authorization chains based on configuration.
///
/// This factory creates a chain of authorizers in the following order:
/// 1. Rule-based authorization (always enabled)
/// 2. Admin authorization (always enabled)
/// 3. Anonymous authorization (if enabled in config)
pub struct AuthzFactory;

impl AuthzFactory {
    /// Creates a new authorization factory.
    pub fn new() -> Self {
        Self
    }

    /// Builds an authorization chain based on the provided configuration.
    ///
    /// # Arguments
    /// * `cfg` - Authorization configuration
    /// * `db` - Database connection for role-based authorization
    ///
    /// # Returns
    /// * A new ChainAuthorizer configured with the appropriate authorizers
    pub fn build_authorizer(&self, cfg: &AuthzConfig, db: Arc<Database>) -> ChainAuthorizer {
        let mut authorizers = vec![];

        let rule_authz = RuleAuthorizer::new(db);
        authorizers.push(UnionAuthorizer::Rule(rule_authz));

        let admin_authz = AdminAuthorizer::new();
        authorizers.push(UnionAuthorizer::Admin(admin_authz));

        if !cfg.anonymous_rules.is_empty() {
            info!(
                "Anonymous authorization is enabled with {} rules",
                cfg.anonymous_rules.len()
            );
            let anonymous_authz = AnonymousAuthorizer::new(cfg.anonymous_rules.clone());
            authorizers.push(UnionAuthorizer::Anonymous(anonymous_authz));
        }

        ChainAuthorizer::new(authorizers)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::CommonConfig;
    use crate::types::user::RoleRule;

    use super::*;

    fn get_chain_length(chain: &ChainAuthorizer) -> usize {
        chain.authorizers.len()
    }

    fn is_anonymous_enabled(chain: &ChainAuthorizer) -> bool {
        chain
            .authorizers
            .iter()
            .any(|auth| matches!(auth, UnionAuthorizer::Anonymous(_)))
    }

    fn is_admin_enabled(chain: &ChainAuthorizer) -> bool {
        chain
            .authorizers
            .iter()
            .any(|auth| matches!(auth, UnionAuthorizer::Admin(_)))
    }

    fn is_rule_enabled(chain: &ChainAuthorizer) -> bool {
        chain
            .authorizers
            .iter()
            .any(|auth| matches!(auth, UnionAuthorizer::Rule(_)))
    }

    #[test]
    fn test_factory() {
        let factory = AuthzFactory::new();
        let db = Arc::new(Database::new_test());

        // Test default config (no anonymous rules)
        let cfg = AuthzConfig::default();
        let chain = factory.build_authorizer(&cfg, db.clone());
        assert_eq!(get_chain_length(&chain), 2);
        assert!(!is_anonymous_enabled(&chain));
        assert!(is_admin_enabled(&chain));
        assert!(is_rule_enabled(&chain));

        // Test with anonymous rules enabled
        let mut cfg = AuthzConfig::default();
        cfg.anonymous_rules = vec![RoleRule {
            resources: vec!["public_resource".to_string()].into_iter().collect(),
            verbs: vec!["read".to_string()].into_iter().collect(),
        }];
        let chain = factory.build_authorizer(&cfg, db.clone());
        assert_eq!(get_chain_length(&chain), 3);
        assert!(is_anonymous_enabled(&chain));
        assert!(is_admin_enabled(&chain));
        assert!(is_rule_enabled(&chain));

        // Verify authorizer order
        if let Some(UnionAuthorizer::Rule(_)) = chain.authorizers.first() {
            // Rule authorizer should be first
        } else {
            panic!("First authorizer should be Rule");
        }
        if let Some(UnionAuthorizer::Admin(_)) = chain.authorizers.get(1) {
            // Admin authorizer should be second
        } else {
            panic!("Second authorizer should be Admin");
        }
        if let Some(UnionAuthorizer::Anonymous(_)) = chain.authorizers.get(2) {
            // Anonymous authorizer should be last
        } else {
            panic!("Third authorizer should be Anonymous");
        }
    }
}
