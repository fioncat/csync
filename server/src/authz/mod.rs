mod admin;
mod anonymous;
mod rule;
mod union;

pub mod chain;
pub mod config;
pub mod factory;

use anyhow::Result;

use super::authn::AuthnUserInfo;

/// Trait that defines the authorization interface
///
/// Implementers of this trait can authorize requests based on custom logic.
/// The trait is thread-safe and can be shared across threads.
pub trait Authorizer: Send + Sync {
    /// Authorizes a request and returns an AuthzResponse
    ///
    /// # Arguments
    /// * `req` - The authorization request containing resource, verb and user info
    ///
    /// # Returns
    /// * `Result<AuthzResponse>` - The authorization decision wrapped in a Result
    fn authorize_request(&self, req: &AuthzRequest) -> Result<AuthzResponse>;
}

/// Represents an authorization request that needs to be validated
///
/// Contains all the necessary information to make an authorization decision:
/// - The resource being accessed
/// - The verb/action being performed
/// - The user making the request
#[derive(Debug, Clone)]
pub struct AuthzRequest {
    /// The resource identifier that is being accessed
    pub resource: String,
    /// The action/verb being performed on the resource
    pub verb: String,
    /// Information about the authenticated user making the request
    pub user: AuthnUserInfo,
}

/// Possible responses from an authorization check.
#[derive(Debug, Copy, Clone)]
pub enum AuthzResponse {
    /// Access is granted - the request is authorized
    Ok,
    /// Defers decision to next authorizer in chain - current authorizer cannot make a decision
    Continue,
    /// Access is denied - the request is not authorized
    Unauthorized,
}
