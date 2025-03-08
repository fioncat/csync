use std::path::PathBuf;

use anyhow::{bail, Result};
use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Root};
use log4rs::Config;
use serde::{Deserialize, Serialize};

use crate::config::{CommonConfig, PathSet};
use crate::dirs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LogsConfig {
    #[serde(default = "LogTarget::default")]
    pub target: LogTarget,

    #[serde(default = "LogLevel::default")]
    pub level: LogLevel,

    #[serde(default = "LogsConfig::default_file_archive")]
    pub file_archive: u32,

    #[serde(default = "LogsConfig::default_file_max_size_mib")]
    pub file_max_size_mib: u64,

    #[serde(skip)]
    logs_dir: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default)]
pub enum LogTarget {
    #[serde(rename = "stdout")]
    #[default]
    Stdout,

    #[serde(rename = "stderr")]
    Stderr,

    #[serde(rename = "file")]
    File,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default)]
pub enum LogLevel {
    #[serde(rename = "info")]
    #[default]
    Info,

    #[serde(rename = "error")]
    Error,

    #[serde(rename = "warning")]
    Warning,
}

impl CommonConfig for LogsConfig {
    fn complete(&mut self, ps: &PathSet) -> Result<()> {
        if !matches!(self.target, LogTarget::File) {
            return Ok(());
        }

        if self.file_archive == 0 {
            bail!("file_archive must be greater than 0");
        }
        if self.file_max_size_mib == 0 {
            bail!("file_max_size_mib must be greater than 0");
        }

        self.logs_dir = ps.data_dir.join("logs");
        dirs::ensure_dir_exists(&self.logs_dir)?;

        Ok(())
    }
}

impl Default for LogsConfig {
    fn default() -> Self {
        LogsConfig {
            target: LogTarget::default(),
            level: LogLevel::default(),
            file_archive: LogsConfig::default_file_archive(),
            file_max_size_mib: LogsConfig::default_file_max_size_mib(),
            logs_dir: PathBuf::new(),
        }
    }
}

impl LogsConfig {
    pub fn init(&self, name: &str) -> Result<()> {
        let level_filter = match self.level {
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warning => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
        };

        let config = match self.target {
            LogTarget::Stdout => {
                let stdout = ConsoleAppender::builder().build();

                Config::builder()
                    .appender(Appender::builder().build("stdout", Box::new(stdout)))
                    .build(Root::builder().appender("stdout").build(level_filter))?
            }
            LogTarget::Stderr => {
                let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

                Config::builder()
                    .appender(Appender::builder().build("stderr", Box::new(stderr)))
                    .build(Root::builder().appender("stderr").build(level_filter))?
            }
            LogTarget::File => {
                let path = self.logs_dir.join(format!("{name}.log"));

                let archived_pattern = self.logs_dir.join(format!("{name}.{{}}.log"));
                let archived_pattern = format!("{}", archived_pattern.display());

                let window_roller = FixedWindowRoller::builder()
                    .base(1)
                    .build(&archived_pattern, self.file_archive)?;

                let size_trigger = SizeTrigger::new(self.file_max_size_mib * 1024 * 1024);

                let compound_policy =
                    CompoundPolicy::new(Box::new(size_trigger), Box::new(window_roller));

                let file_appender =
                    RollingFileAppender::builder().build(path, Box::new(compound_policy))?;

                Config::builder()
                    .appender(Appender::builder().build("file", Box::new(file_appender)))
                    .build(Root::builder().appender("file").build(level_filter))?
            }
        };

        log4rs::init_config(config)?;

        Ok(())
    }

    fn default_file_archive() -> u32 {
        5
    }

    fn default_file_max_size_mib() -> u64 {
        10
    }
}
