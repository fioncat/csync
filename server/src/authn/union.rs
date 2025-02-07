use actix_web::HttpRequest;
use anyhow::Result;

use super::admin::AdminAuthenticator;
use super::anonymous::AnonymousAuthenticator;
use super::bearer_token::BearerTokenAuthenticator;
use super::token::TokenValidator;
use super::{Authenticator, AuthnResponse, AuthnUserInfo};

/// Union type that combines different authenticator implementations.
pub enum UnionAuthenticator<T: TokenValidator> {
    /// Bearer token authentication using the Authorization header
    BearerToken(BearerTokenAuthenticator<T>),
    /// Admin privilege validation based on IP allow list
    Admin(AdminAuthenticator),
    /// Anonymous access fallback
    Anonymous(AnonymousAuthenticator),
}

impl<T: TokenValidator + Sync + Send> Authenticator for UnionAuthenticator<T> {
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        match self {
            UnionAuthenticator::BearerToken(auth) => auth.authenticate_request(req, user),
            UnionAuthenticator::Admin(auth) => auth.authenticate_request(req, user),
            UnionAuthenticator::Anonymous(auth) => auth.authenticate_request(req, user),
        }
    }
}
