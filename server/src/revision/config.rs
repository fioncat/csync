use anyhow::Result;
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RevisionConfig {
    #[serde(default = "RevisionConfig::default_name")]
    pub name: RevisionType,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum RevisionType {
    #[serde(rename = "memory")]
    Memory,
}

impl CommonConfig for RevisionConfig {
    fn default() -> Self {
        Self {
            name: Self::default_name(),
        }
    }
    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        Ok(())
    }
}

impl RevisionConfig {
    fn default_name() -> RevisionType {
        RevisionType::Memory
    }
}
