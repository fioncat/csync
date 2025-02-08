use std::collections::HashSet;

use anyhow::Result;
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

use super::token::config::TokenConfig;

/// Authentication configuration that controls various authentication mechanisms.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthnConfig {
    /// List of IP addresses allowed to access admin endpoints.
    /// Default: ["127.0.0.1"]
    ///
    /// Special value "*" can be used to allow admin access from any IP address.
    /// WARNING: Using "*" is highly discouraged in production as it significantly
    /// increases security risks. Admin users have full access to modify or delete
    /// any data in the database.
    ///
    /// If empty, admin authentication will be disabled completely.
    #[serde(default = "AuthnConfig::default_admin_allow_list")]
    pub admin_allow_list: HashSet<String>,

    /// Password for the admin user.
    /// Default: "admin"
    ///
    /// WARNING: The default password is extremely insecure and should NEVER be used
    /// in production environments. Admin users have unrestricted access to all data
    /// and operations. Please configure a strong password.
    ///
    /// If empty, admin authentication will be disabled completely.
    #[serde(default = "AuthnConfig::default_admin_password")]
    pub admin_password: String,

    /// Whether to allow anonymous access when no authentication is provided.
    /// Default: false
    #[serde(default = "AuthnConfig::default_allow_anonymous")]
    pub allow_anonymous: bool,

    /// Token-based authentication configuration.
    /// See TokenConfig for details.
    #[serde(default = "TokenConfig::default")]
    pub token: TokenConfig,
}

impl CommonConfig for AuthnConfig {
    fn default() -> Self {
        Self {
            admin_allow_list: Self::default_admin_allow_list(),
            admin_password: Self::default_admin_password(),
            allow_anonymous: Self::default_allow_anonymous(),
            token: TokenConfig::default(),
        }
    }

    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        self.token.complete(ps)?;
        Ok(())
    }
}

impl AuthnConfig {
    pub fn default_admin_allow_list() -> HashSet<String> {
        vec![String::from("127.0.0.1")].into_iter().collect()
    }

    pub fn default_allow_anonymous() -> bool {
        false
    }

    pub fn default_admin_password() -> String {
        String::from("admin")
    }
}
