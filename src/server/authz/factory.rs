use std::sync::Arc;

use crate::server::authz::anonymous::AnonymousAuthorizer;
use crate::server::db::Database;

use super::admin::AdminAuthorizer;
use super::chain::ChainAuthorizer;
use super::config::AuthzConfig;
use super::rule::RuleAuthorizer;
use super::union::UnionAuthorizer;

pub struct AuthzFactory;

impl AuthzFactory {
    pub fn new() -> Self {
        Self
    }

    pub fn build_authorizer(&self, cfg: &AuthzConfig, db: Arc<Database>) -> ChainAuthorizer {
        let mut authorizers = vec![];

        let rule_authz = RuleAuthorizer::new(db);
        authorizers.push(UnionAuthorizer::Rule(rule_authz));

        let admin_authz = AdminAuthorizer::new();
        authorizers.push(UnionAuthorizer::Admin(admin_authz));

        if !cfg.anonymous_rules.is_empty() {
            let anonymous_authz = AnonymousAuthorizer::new(cfg.anonymous_rules.clone());
            authorizers.push(UnionAuthorizer::Anonymous(anonymous_authz));
        }

        ChainAuthorizer::new(authorizers)
    }
}
