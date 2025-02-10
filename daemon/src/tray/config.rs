use anyhow::{bail, Context, Result};
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TrayConfig {
    #[serde(default = "TrayConfig::default_enable")]
    pub enable: bool,

    #[serde(default = "TrayConfig::default_allow_save")]
    pub allow_save: bool,

    #[serde(default = "TrayConfig::default_truncate_text")]
    pub truncate_text: usize,

    #[serde(default = "TrayConfig::default_text")]
    pub text: ResourceConfig,

    #[serde(default = "TrayConfig::default_image")]
    pub image: ResourceConfig,

    #[serde(default = "TrayConfig::default_file")]
    pub file: ResourceConfig,
}

impl CommonConfig for TrayConfig {
    fn default() -> Self {
        Self {
            enable: Self::default_enable(),
            allow_save: Self::default_allow_save(),
            truncate_text: Self::default_truncate_text(),
            text: Self::default_text(),
            image: Self::default_image(),
            file: Self::default_file(),
        }
    }

    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        if self.truncate_text < Self::MIN_TRUNCATE_TEXT
            || self.truncate_text > Self::MAX_TRUNCATE_TEXT
        {
            bail!(
                "invalid truncate_text: {}, should be in range [{}, {}]",
                self.truncate_text,
                Self::MIN_TRUNCATE_TEXT,
                Self::MAX_TRUNCATE_TEXT
            );
        }

        if !self.text.enable && !self.image.enable && !self.file.enable {
            bail!("all resources in tray are disabled");
        }

        self.text.validate().context("validate text")?;
        self.image.validate().context("validate image")?;
        self.file.validate().context("validate file")?;

        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResourceConfig {
    #[serde(default = "ResourceConfig::default_enable")]
    pub enable: bool,

    pub limit: u64,
}

impl TrayConfig {
    const MIN_TRUNCATE_TEXT: usize = 5;
    const MAX_TRUNCATE_TEXT: usize = 100;

    fn default_enable() -> bool {
        true
    }

    fn default_truncate_text() -> usize {
        60
    }

    fn default_allow_save() -> bool {
        true
    }

    fn default_text() -> ResourceConfig {
        ResourceConfig {
            enable: true,
            limit: 20,
        }
    }

    fn default_image() -> ResourceConfig {
        ResourceConfig {
            enable: true,
            limit: 5,
        }
    }

    fn default_file() -> ResourceConfig {
        ResourceConfig {
            enable: true,
            limit: 5,
        }
    }
}

impl ResourceConfig {
    const MAX_LIMIT: u64 = 50;
    const MIN_LIMIT: u64 = 1;

    fn default_enable() -> bool {
        true
    }

    fn validate(&self) -> Result<()> {
        if self.limit < Self::MIN_LIMIT || self.limit > Self::MAX_LIMIT {
            bail!(
                "invalid limit: {}, should be in range [{}, {}]",
                self.limit,
                Self::MIN_LIMIT,
                Self::MAX_LIMIT
            );
        }
        Ok(())
    }
}
