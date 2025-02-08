use actix_web::HttpRequest;
use anyhow::Result;

use super::token::TokenValidator;
use super::union::UnionAuthenticator;
use super::{Authenticator, AuthnResponse, AuthnUserInfo};

/// Chain of authenticators that processes authentication requests sequentially.
///
/// This authenticator chains multiple authenticators together and processes them in order.
/// Each authenticator in the chain can:
/// - Pass through (Continue) to the next authenticator
/// - Authenticate the user (Ok) and pass to next authenticator for additional processing
/// - Reject the authentication (Unauthenticated) and stop the chain
///
/// The chain succeeds if any authenticator succeeds and no subsequent authenticator rejects.
/// The last successful authentication result is returned.
///
/// # Type Parameters
/// * `T` - Token validator implementation used by authenticators in the chain
pub struct ChainAuthenticator<T: TokenValidator> {
    pub(super) authenticators: Vec<UnionAuthenticator<T>>,
}

impl<T: TokenValidator> ChainAuthenticator<T> {
    /// Creates a new authentication chain with the specified authenticators.
    ///
    /// The authenticators are processed in order, with each authenticator potentially
    /// modifying or replacing the authentication result from previous authenticators.
    ///
    /// # Arguments
    /// * `authenticators` - Vector of authenticators to chain together
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

#[cfg(test)]
mod tests {
    use actix_web::test::TestRequest;
    use std::collections::HashSet;

    use crate::authn::admin::AdminAuthenticator;
    use crate::authn::anonymous::AnonymousAuthenticator;
    use crate::authn::bearer_token::BearerTokenAuthenticator;
    use crate::authn::token::simple::SimpleToken;
    use crate::authn::union::UnionAuthenticator;

    use super::*;

    #[test]
    fn test_chain() {
        // Create individual authenticators
        let token_auth = BearerTokenAuthenticator::new(SimpleToken::new());
        let mut allow_list = HashSet::new();
        allow_list.insert("127.0.0.1".to_string());
        let admin_auth = AdminAuthenticator::new(allow_list);
        let anon_auth = AnonymousAuthenticator::new();

        // Create chain
        let authenticators = vec![
            UnionAuthenticator::BearerToken(token_auth),
            UnionAuthenticator::Admin(admin_auth),
            UnionAuthenticator::Anonymous(anon_auth),
        ];
        let chain = ChainAuthenticator::new(authenticators);

        // Test anonymous fallback
        let req = TestRequest::default().to_http_request();
        let resp = chain.authenticate_request(&req, None).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert!(user.name.is_empty());
                assert!(user.is_anonymous);
                assert!(!user.is_admin);
            }
            _ => panic!("expected anonymous user"),
        }

        // Test valid token auth
        let req = TestRequest::default()
            .insert_header(("Authorization", "Bearer simple-token-alice"))
            .to_http_request();
        let resp = chain.authenticate_request(&req, None).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "alice");
                assert!(!user.is_anonymous);
                assert!(!user.is_admin);
            }
            _ => panic!("expected authenticated user"),
        }

        // Test admin privileges
        let req = TestRequest::default()
            .peer_addr("127.0.0.1:1234".parse().unwrap())
            .insert_header(("Authorization", "Bearer simple-token-admin"))
            .to_http_request();
        let resp = chain.authenticate_request(&req, None).unwrap();
        match resp {
            AuthnResponse::Ok(user) => {
                assert_eq!(user.name, "admin");
                assert!(!user.is_anonymous);
                assert!(user.is_admin);
            }
            _ => panic!("expected admin user"),
        }

        // Test admin rejection
        let req = TestRequest::default()
            .peer_addr("192.168.1.1:1234".parse().unwrap())
            .insert_header(("Authorization", "Bearer simple-token-admin"))
            .to_http_request();
        let resp = chain.authenticate_request(&req, None).unwrap();
        assert!(matches!(resp, AuthnResponse::Unauthenticated));
    }
}
