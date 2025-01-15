use anyhow::Result;

use super::{Authorizer, AuthzRequest, AuthzResponse};

pub struct AdminAuthorizer;

impl AdminAuthorizer {
    pub fn new() -> Self {
        Self
    }
}

impl Authorizer for AdminAuthorizer {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse> {
        if req.user.is_admin {
            return Ok(AuthzResponse::Ok);
        }

        Ok(AuthzResponse::Continue)
    }
}
