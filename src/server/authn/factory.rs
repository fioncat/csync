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

pub struct AuthnFactory;

impl AuthnFactory {
    pub fn new() -> Self {
        Self
    }

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
