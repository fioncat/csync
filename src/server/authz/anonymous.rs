use anyhow::Result;

use crate::types::user::RoleRule;

use super::rule::is_authorized;
use super::{Authorizer, AuthzRequest, AuthzResponse};

pub struct AnonymousAuthorizer {
    rules: Vec<RoleRule>,
}

impl AnonymousAuthorizer {
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
