use anyhow::{bail, Context, Result};
use csync_misc::config::{CommonConfig, PathSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecycleConfig {
    #[serde(default = "RecycleConfig::default_text")]
    pub text: RecycleResourceConfig,

    #[serde(default = "RecycleConfig::default_image")]
    pub image: RecycleResourceConfig,

    #[serde(default = "RecycleConfig::default_file")]
    pub file: RecycleResourceConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RecycleResourceConfig {
    pub enable: bool,
    pub max: u64,
    pub keep_hours: u64,
}

impl CommonConfig for RecycleConfig {
    fn default() -> Self {
        Self {
            text: Self::default_text(),
            image: Self::default_image(),
            file: Self::default_file(),
        }
    }

    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        self.text.validate().context("text")?;
        self.image.validate().context("image")?;
        self.file.validate().context("file")?;
        Ok(())
    }
}

impl RecycleConfig {
    pub fn default_text() -> RecycleResourceConfig {
        RecycleResourceConfig {
            enable: true,
            max: 100,
            keep_hours: 24,
        }
    }

    pub fn default_image() -> RecycleResourceConfig {
        RecycleResourceConfig {
            enable: true,
            max: 5,
            keep_hours: 2,
        }
    }

    pub fn default_file() -> RecycleResourceConfig {
        RecycleResourceConfig {
            enable: true,
            max: 10,
            keep_hours: 5,
        }
    }
}

impl RecycleResourceConfig {
    const MAX_KEEP: u64 = 2000;
    const MIN_KEEP: u64 = 10;
    const MAX_KEEP_HOURS: u64 = 10 * 24;

    pub fn validate(&self) -> Result<()> {
        if !self.enable {
            return Ok(());
        }

        if self.max < Self::MIN_KEEP {
            bail!("max must be at least {}", Self::MIN_KEEP);
        }
        if self.max > Self::MAX_KEEP {
            bail!("max must be at most {}", Self::MAX_KEEP);
        }

        if self.keep_hours > Self::MAX_KEEP_HOURS {
            bail!("keep_hours must be at most {}", Self::MAX_KEEP_HOURS);
        }
        if self.keep_hours == 0 {
            bail!("keep_hours must be at least 1");
        }

        Ok(())
    }
}
