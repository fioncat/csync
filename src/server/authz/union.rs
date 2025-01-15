use anyhow::Result;

use super::admin::AdminAuthorizer;
use super::anonymous::AnonymousAuthorizer;
use super::rule::RuleAuthorizer;
use super::{Authorizer, AuthzRequest, AuthzResponse};

pub enum UnionAuthorizer {
    Admin(AdminAuthorizer),
    Rule(RuleAuthorizer),
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
