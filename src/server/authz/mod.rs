mod admin;
mod anonymous;
mod rule;
mod union;

pub mod chain;
pub mod config;
pub mod factory;

use anyhow::Result;

use super::authn::AuthnUserInfo;

pub trait Authorizer: Send + Sync {
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse>;
}

#[derive(Debug, Clone)]
pub struct AuthzRequest {
    pub resource: String,
    pub verb: String,
    pub user: AuthnUserInfo,
}

/// Possible responses from an authorization check.
#[derive(Debug, Copy, Clone)]
pub enum AuthzResponse {
    /// Access is granted
    Ok,
    /// Defers decision to next authorizer in chain
    Continue,
    /// Access is denied
    Unauthorized,
}
