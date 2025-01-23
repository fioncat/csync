use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};
use crate::types::user::RoleRule;

/// Authorization related configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthzConfig {
    /// Rules for anonymous authorization. These rules will be used when the request
    /// is not authenticated. If empty, anonymous access is not allowed.
    /// Defaults to empty.
    #[serde(default = "AuthzConfig::default_anonymous_rules")]
    pub anonymous_rules: Vec<RoleRule>,
}

impl CommonConfig for AuthzConfig {
    fn default() -> Self {
        Self {
            anonymous_rules: Self::default_anonymous_rules(),
        }
    }

    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        Ok(())
    }
}

impl AuthzConfig {
    pub fn default_anonymous_rules() -> Vec<RoleRule> {
        vec![]
    }
}
