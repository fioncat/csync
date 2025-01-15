use actix_web::HttpRequest;
use anyhow::Result;

use super::token::TokenValidator;
use super::union::UnionAuthenticator;
use super::{Authenticator, AuthnResponse, AuthnUserInfo};

pub struct ChainAuthenticator<T: TokenValidator> {
    authenticators: Vec<UnionAuthenticator<T>>,
}

impl<T: TokenValidator> ChainAuthenticator<T> {
    pub fn new(authenticators: Vec<UnionAuthenticator<T>>) -> Self {
        Self { authenticators }
    }
}

impl<T: TokenValidator + Sync + Send> Authenticator for ChainAuthenticator<T> {
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        mut user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        for authenticator in self.authenticators.iter() {
            let old_user = user.take();
            let resp = authenticator.authenticate_request(req, old_user)?;
            match resp {
                AuthnResponse::Ok(new_user) => user = Some(new_user),
                AuthnResponse::Continue => continue,
                AuthnResponse::Unauthenticated => return Ok(AuthnResponse::Unauthenticated),
            }
        }
        match user {
            Some(user) => Ok(AuthnResponse::Ok(user)),
            None => Ok(AuthnResponse::Continue),
        }
    }
}
