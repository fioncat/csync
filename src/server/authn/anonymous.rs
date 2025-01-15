use actix_web::HttpRequest;
use anyhow::Result;

use super::{Authenticator, AuthnResponse, AuthnUserInfo};

/// Authenticator that provides anonymous access when no other authentication is available.
///
/// This authenticator acts as a fallback mechanism in the authentication chain.
/// If a user is already authenticated, it preserves their identity.
/// Otherwise, it creates an anonymous user with limited privileges.
pub struct AnonymousAuthenticator;

impl AnonymousAuthenticator {
    /// Creates a new AnonymousAuthenticator.
    pub fn new() -> Self {
        Self {}
    }
}

impl Authenticator for AnonymousAuthenticator {
    fn authenticate_request(
        &self,
        _req: &HttpRequest,
        user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse> {
        if let Some(user) = user {
            return Ok(AuthnResponse::Ok(user));
        }

        Ok(AuthnResponse::Ok(AuthnUserInfo {
            name: String::new(),
            is_admin: false,
            is_anonymous: true,
        }))
    }
}
