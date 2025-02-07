use anyhow::Result;

use super::admin::AdminAuthorizer;
use super::anonymous::AnonymousAuthorizer;
use super::rule::RuleAuthorizer;
use super::{Authorizer, AuthzRequest, AuthzResponse};

/// A union type that can hold different types of authorizers
pub enum UnionAuthorizer {
    /// Handles authorization for admin users
    Admin(AdminAuthorizer),
    /// Handles authorization based on user roles and rules
    Rule(RuleAuthorizer),
    /// Handles authorization for anonymous users
    Anonymous(AnonymousAuthorizer),
}

impl Authorizer for UnionAuthorizer {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse> {
        match self {
            UnionAuthorizer::Admin(a) => a.authorize_request(req),
            UnionAuthorizer::Rule(r) => r.authorize_request(req),
            UnionAuthorizer::Anonymous(a) => a.authorize_request(req),
        }
    }
}
