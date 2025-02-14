pub mod config;

use std::path::Path;

use anyhow::Result;
use config::{LogConfig, LogLevel, LogName};
use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::append::rolling_file::policy::compound::roll::fixed_window::FixedWindowRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Root};
use log4rs::Config;

pub fn init_logger(dir: &Path, name: &str, cfg: &LogConfig) -> Result<()> {
    let level_filter = match cfg.level {
        LogLevel::Error => LevelFilter::Error,
        LogLevel::Warning => LevelFilter::Warn,
        LogLevel::Info => LevelFilter::Info,
    };
    let config = match cfg.name {
        LogName::Stdout => {
            let stdout = ConsoleAppender::builder().build();

            Config::builder()
                .appender(Appender::builder().build("stdout", Box::new(stdout)))
                .build(Root::builder().appender("stdout").build(level_filter))?
        }
        LogName::Stderr => {
            let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

            Config::builder()
                .appender(Appender::builder().build("stderr", Box::new(stderr)))
                .build(Root::builder().appender("stderr").build(level_filter))?
        }
        LogName::File => {
            let path = dir.join(format!("{name}.log"));

            let archived_pattern = dir.join(format!("{name}.{{}}.log"));
            let archived_pattern = format!("{}", archived_pattern.display());

            let window_roller = FixedWindowRoller::builder()
                .base(1)
                .build(&archived_pattern, cfg.file_archive)?;

            let size_trigger = SizeTrigger::new(cfg.file_max_size_mib * 1024 * 1024);

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
