use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LogConfig {
    #[serde(default = "LogConfig::default_name")]
    pub name: LogName,

    #[serde(default = "LogConfig::default_level")]
    pub level: LogLevel,

    #[serde(default = "LogConfig::default_file_archive")]
    pub file_archive: u32,

    #[serde(default = "LogConfig::default_file_max_size_mib")]
    pub file_max_size_mib: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum LogName {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
    #[serde(rename = "file")]
    File,
}

impl CommonConfig for LogConfig {
    fn default() -> Self {
        Self {
            name: Self::default_name(),
            level: Self::default_level(),
            file_archive: Self::default_file_archive(),
            file_max_size_mib: Self::default_file_max_size_mib(),
        }
    }

    fn complete(&mut self, _ps: &PathSet) -> Result<()> {
        if matches!(self.name, LogName::File) {
            if self.file_archive < Self::MIN_FILE_ARCHIVE {
                bail!(
                    "file_archive must be greater than or equal to {}",
                    Self::MIN_FILE_ARCHIVE
                );
            }
            if self.file_archive > Self::MAX_FILE_ARCHIVE {
                bail!(
                    "file_archive must be less than or equal to {}",
                    Self::MAX_FILE_ARCHIVE
                );
            }
            if self.file_max_size_mib < Self::MIN_FILE_MAX_SIZE_MIB {
                bail!(
                    "file_max_size_mib must be greater than or equal to {}",
                    Self::MIN_FILE_MAX_SIZE_MIB
                );
            }
            if self.file_max_size_mib > Self::MAX_FILE_MAX_SIZE_MIB {
                bail!(
                    "file_max_size_mib must be less than or equal to {}",
                    Self::MAX_FILE_MAX_SIZE_MIB
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum LogLevel {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warning")]
    Warning,
}

impl LogConfig {
    const MAX_FILE_ARCHIVE: u32 = 1000;
    const MIN_FILE_ARCHIVE: u32 = 2;

    const MAX_FILE_MAX_SIZE_MIB: u64 = 5 * 1024; // 5 GiB
    const MIN_FILE_MAX_SIZE_MIB: u64 = 1; // 1 MiB

    pub fn default_name() -> LogName {
        LogName::Stdout
    }

    pub fn default_level() -> LogLevel {
        LogLevel::Info
    }

    pub fn default_file_archive() -> u32 {
        5
    }

    pub fn default_file_max_size_mib() -> u64 {
        10
    }
}
