use std::io::{self, IsTerminal};

use anyhow::{bail, Context, Result};
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;

/// Initializes the logging system with the specified log level.
///
/// Sets up a logging system that:
/// - Formats logs with timestamps in RFC3339 format
/// - Uses colors for log levels when outputting to a terminal
/// - Writes logs to stdout
///
/// # Arguments
/// * `level` - The log level to use. Valid values are:
///   - "error": Only show error messages
///   - "info": Show info and error messages
///   - "debug": Show debug, info, and error messages
///
/// # Returns
/// * `Ok(())` if initialization succeeds
/// * `Err` if initialization fails or if an invalid log level is provided
///
/// # Examples
/// ```
/// use crate::logs;
///
/// // Initialize with info level
/// logs::init("info").expect("Failed to initialize logger");
///
/// // Initialize with debug level
/// logs::init("debug").expect("Failed to initialize logger");
/// ```
///
/// # Errors
/// Returns error if:
/// * An invalid log level is provided
/// * Logger initialization fails
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
