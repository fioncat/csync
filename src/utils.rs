use std::fs;
use std::io;
use std::path::Path;

use anyhow::{Context, Result};
use log::info;

/// If the file directory doesn't exist, create it; if it exists, take no action.
pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if let Some(dir) = path.as_ref().parent() {
        match fs::read_dir(dir) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                fs::create_dir_all(dir)
                    .with_context(|| format!("create directory '{}'", dir.display()))?;
                Ok(())
            }
            Err(err) => Err(err).with_context(|| format!("read directory '{}'", dir.display())),
        }
    } else {
        Ok(())
    }
}

pub struct BuildInfo {
    version: &'static str,
    build_type: &'static str,
    build_target: &'static str,
    build_sha: &'static str,
    build_time: &'static str,
}

impl BuildInfo {
    #[inline]
    pub fn new() -> Self {
        Self {
            version: env!("CSYNC_VERSION"),
            build_type: env!("CSYNC_BUILD_TYPE"),
            build_target: env!("CSYNC_TARGET"),
            build_sha: env!("CSYNC_SHA"),
            build_time: env!("VERGEN_BUILD_TIMESTAMP"),
        }
    }

    pub fn log(&self) {
        info!(
            "Welcome to csync, version {} ({}), target '{}', commit '{}'",
            self.version, self.build_type, self.build_target, self.build_sha
        );
    }
}
