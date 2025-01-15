use anyhow::Result;

use super::union::UnionAuthorizer;
use super::{Authorizer, AuthzRequest, AuthzResponse};

pub struct ChainAuthorizer {
    authorizers: Vec<UnionAuthorizer>,
}

impl ChainAuthorizer {
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
