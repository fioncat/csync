use std::collections::HashSet;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};

use super::token::config::TokenConfig;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthnConfig {
    #[serde(default = "AuthnConfig::default_admin_allow_list")]
    pub admin_allow_list: HashSet<String>,

    #[serde(default = "AuthnConfig::default_admin_password")]
    pub admin_password: String,

    #[serde(default = "AuthnConfig::default_allow_anonymous")]
    pub allow_anonymous: bool,

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
