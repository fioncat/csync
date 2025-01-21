use anyhow::Result;
use log::{info, warn};

use crate::server::authn::admin::AdminAuthenticator;
use crate::server::authn::anonymous::AnonymousAuthenticator;

use super::bearer_token::BearerTokenAuthenticator;
use super::chain::ChainAuthenticator;
use super::config::AuthnConfig;
use super::token::factory::TokenFactory;
use super::token::jwt::JwtTokenValidator;
use super::union::UnionAuthenticator;

/// Factory for building authentication chains based on configuration.
///
/// This factory creates a chain of authenticators in the following order:
/// 1. Bearer token authentication (always enabled)
/// 2. Admin authentication (if enabled in config)
/// 3. Anonymous authentication (if enabled in config)
pub struct AuthnFactory;

impl AuthnFactory {
    /// Creates a new authentication factory.
    pub fn new() -> Self {
        Self
    }

    /// Builds an authentication chain based on the provided configuration.
    ///
    /// # Arguments
    /// * `cfg` - Authentication configuration
    /// * `token_factory` - Factory for creating token validators
    pub fn build_authenticator(
        &self,
        cfg: &AuthnConfig,
        token_factory: &TokenFactory,
    ) -> Result<ChainAuthenticator<JwtTokenValidator>> {
        let mut authenticators = Vec::new();

        let jwt = token_factory.build_token_validator()?;
        let token_auth = BearerTokenAuthenticator::new(jwt);
        authenticators.push(UnionAuthenticator::BearerToken(token_auth));

        if cfg.admin_password == "admin" {
            warn!("Using default admin password IS DANGEROUS, please change it in production");
        }
        if !cfg.admin_allow_list.is_empty() && !cfg.admin_password.is_empty() {
            if cfg.admin_allow_list.contains("*") {
                warn!("Allow every IP to authenticate as admin (with '*' in admin_allow_list), this is dangerous");
            }
            let admin_auth = AdminAuthenticator::new(cfg.admin_allow_list.clone());
            authenticators.push(UnionAuthenticator::Admin(admin_auth));
        } else {
            warn!("Admin authentication disabled");
        }

        if cfg.allow_anonymous {
            info!("Anonymous authentication is enabled");
            let anonymous_auth = AnonymousAuthenticator::new();
            authenticators.push(UnionAuthenticator::Anonymous(anonymous_auth));
        }

        let chain = ChainAuthenticator::new(authenticators);
        Ok(chain)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::config::CommonConfig;
    use crate::server::authn::token::config::TokenConfig;

    use super::*;

    fn mock_token_factory() -> TokenFactory {
        let cfg = TokenConfig {
            public_key_path: "testdata/public_key.pem".to_string(),
            private_key_path: "testdata/private_key.pem".to_string(),
            expiry: 3600,
            generate_if_not_exists: false,
        };
        TokenFactory::new(&cfg).unwrap()
    }

    fn get_chain_length(chain: &ChainAuthenticator<JwtTokenValidator>) -> usize {
        chain.authenticators.len()
    }

    fn is_anonymous_enabled(chain: &ChainAuthenticator<JwtTokenValidator>) -> bool {
        chain
            .authenticators
            .iter()
            .any(|auth| matches!(auth, UnionAuthenticator::Anonymous(_)))
    }

    fn is_admin_enabled(chain: &ChainAuthenticator<JwtTokenValidator>) -> bool {
        chain
            .authenticators
            .iter()
            .any(|auth| matches!(auth, UnionAuthenticator::Admin(_)))
    }

    #[test]
    fn test_factory() {
        let factory = AuthnFactory::new();
        let token_factory = mock_token_factory();

        // Test default config
        let cfg = AuthnConfig::default();
        let chain = factory.build_authenticator(&cfg, &token_factory).unwrap();
        assert_eq!(get_chain_length(&chain), 2);
        assert!(!is_anonymous_enabled(&chain));
        assert!(is_admin_enabled(&chain));

        // Test with anonymous enabled
        let mut cfg = AuthnConfig::default();
        cfg.allow_anonymous = true;
        let chain = factory.build_authenticator(&cfg, &token_factory).unwrap();
        assert_eq!(get_chain_length(&chain), 3);
        assert!(is_anonymous_enabled(&chain));
        assert!(is_admin_enabled(&chain));

        // Test with all authenticators enabled
        let mut cfg = AuthnConfig::default();
        cfg.allow_anonymous = true;
        cfg.admin_password = "strong_password".to_string();
        let mut allow_list = HashSet::new();
        allow_list.insert("127.0.0.1".to_string());
        cfg.admin_allow_list = allow_list;
        let chain = factory.build_authenticator(&cfg, &token_factory).unwrap();
        assert_eq!(get_chain_length(&chain), 3);
        assert!(is_admin_enabled(&chain));
        assert!(is_anonymous_enabled(&chain));

        // Test with admin disabled
        let mut cfg = AuthnConfig::default();
        cfg.admin_password = "".to_string();
        cfg.allow_anonymous = true;
        let chain = factory.build_authenticator(&cfg, &token_factory).unwrap();
        assert_eq!(get_chain_length(&chain), 2);
        assert!(!is_admin_enabled(&chain));
        assert!(is_anonymous_enabled(&chain));
    }
}
