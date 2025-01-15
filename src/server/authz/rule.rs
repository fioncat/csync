use std::sync::Arc;

use anyhow::Result;

use crate::server::db::Database;
use crate::types::user::RoleRule;

use super::{Authorizer, AuthzRequest, AuthzResponse};

pub struct RuleAuthorizer {
    db: Arc<Database>,
}

impl RuleAuthorizer {
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
