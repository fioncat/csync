mod admin;
mod anonymous;
mod bearer_token;
mod union;

pub mod chain;
pub mod config;
pub mod factory;
pub mod token;

use actix_web::HttpRequest;
use anyhow::Result;
use csync_misc::types::request::Query;

/// Trait for request authenticators.
///
/// Implementors of this trait can authenticate HTTP requests and optionally
/// chain with other authenticators to provide multiple authentication methods.
pub trait Authenticator: Send + Sync {
    /// Attempts to authenticate a request.
    ///
    /// # Arguments
    ///
    /// * `req` - The HTTP request to authenticate
    /// * `user` - Optional user info from previous authentication attempts
    ///
    /// # Returns
    ///
    /// * `Ok(Response::Ok(user))` - Authentication successful with user info
    /// * `Ok(Response::Continue)` - Authentication skipped, try next authenticator
    /// * `Ok(Response::Unauthenticated)` - Authentication failed
    /// * `Err(_)` - Internal error during authentication
    fn authenticate_request(
        &self,
        req: &HttpRequest,
        user: Option<AuthnUserInfo>,
    ) -> Result<AuthnResponse>;
}

/// Response from an authentication attempt.
#[derive(Debug)]
pub enum AuthnResponse {
    /// Authentication successful, contains authenticated user information
    Ok(AuthnUserInfo),
    /// Authentication skipped, should try next authenticator
    Continue,
    /// Authentication failed, should stop authentication chain
    Unauthenticated,
}

/// Information about an authenticated user.
#[derive(Debug, Clone)]
pub struct AuthnUserInfo {
    /// User identifier
    pub name: String,
    /// Whether the user has administrator privileges
    pub is_admin: bool,
    /// Whether this is an anonymous user
    pub is_anonymous: bool,
}

impl AuthnUserInfo {
    pub fn get_query_owner(&self) -> Option<&str> {
        if self.is_admin {
            return None;
        }
        Some(&self.name)
    }

    pub fn set_query_owner(&self, query: &mut Query) {
        if self.is_admin {
            return;
        }
        query.owner = Some(self.name.clone());
    }
}
