use std::io::{self, IsTerminal};

use anyhow::{bail, Context, Result};
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;

pub fn init(level: &str) -> Result<()> {
    let level = match level {
        "error" => LevelFilter::Error,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        _ => bail!("unknown log level '{}'", level),
    };

    let stdout = io::stdout();
    let is_terminal = stdout.is_terminal();

    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .debug(Color::Magenta);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            if is_terminal {
                out.finish(format_args!(
                    "{} [{}] {}",
                    humantime::format_rfc3339_millis(std::time::SystemTime::now()),
                    colors.color(record.level()),
                    message
                ))
            } else {
                out.finish(format_args!(
                    "{} [{}] {}",
                    humantime::format_rfc3339_millis(std::time::SystemTime::now()),
                    record.level(),
                    message
                ))
            }
        })
        .level(level)
        .chain(std::io::stdout())
        .apply()
        .context("init logger")?;

    Ok(())
}
