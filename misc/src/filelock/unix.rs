use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

use anyhow::{bail, Context, Result};
use log::warn;

/// A global file-based lock to ensure only one instance of a process is running.
///
/// This lock writes the current process ID to a file and maintains an exclusive lock
/// on that file until the `GlobalLock` instance is dropped.
pub struct FileLock {
    path: PathBuf,
    /// Wrap the `file_lock` crate
    _file_lock: file_lock::FileLock,
}

impl FileLock {
    /// Error code returned by the OS when a file lock cannot be acquired
    #[cfg(target_os = "linux")]
    const RESOURCE_TEMPORARILY_UNAVAILABLE_CODE: i32 = 11;

    #[cfg(target_os = "macos")]
    const RESOURCE_TEMPORARILY_UNAVAILABLE_CODE: i32 = 35;

    /// Attempts to acquire a global file lock without blocking.
    ///
    /// This method tries to create or open a lock file at the specified path and acquire an exclusive
    /// lock on it. If successful, it writes the current process ID to the file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the lock file should be created or already exists
    ///
    /// # Returns
    ///
    /// * `Ok(Some(GlobalLock))` - If the lock was successfully acquired
    /// * `Ok(None)` - If another process already holds the lock
    /// * `Err` - If there was an error creating, opening, or writing to the lock file
    pub fn try_acquire(path: PathBuf) -> Result<Option<Self>> {
        let lock_opts = match fs::metadata(&path) {
            Ok(_) => file_lock::FileOptions::new().write(true).read(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => file_lock::FileOptions::new()
                .write(true)
                .read(true)
                .create_new(true)
                .truncate(true),
            Err(e) => return Err(e).context("get lock file metadata error"),
        };
        let mut file_lock = match file_lock::FileLock::lock(&path, false, lock_opts) {
            Ok(lock) => lock,
            Err(err) => match err.raw_os_error() {
                Some(code) if code == Self::RESOURCE_TEMPORARILY_UNAVAILABLE_CODE => {
                    return Ok(None);
                }
                _ => {
                    return Err(err).context("acquire file lock error");
                }
            },
        };

        // Write current pid to file lock.
        let pid = process::id();
        let pid = format!("{pid}");

        file_lock
            .file
            .write_all(pid.as_bytes())
            .with_context(|| format!("write pid to lock file {}", path.display()))?;
        file_lock
            .file
            .flush()
            .with_context(|| format!("flush pid to lock file {}", path.display()))?;

        // The file lock will be released after file_lock dropped.
        Ok(Some(FileLock {
            path,
            _file_lock: file_lock,
        }))
    }

    /// Acquires a global file lock, failing if another process already holds the lock.
    ///
    /// This method is a wrapper around `try_acquire` that returns an error instead of `None`
    /// when the lock is already held by another process.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the lock file should be created or already exists
    ///
    /// # Returns
    ///
    /// * `Ok(GlobalLock)` - If the lock was successfully acquired
    /// * `Err` - If another process holds the lock or if there was an error with the lock file
    pub fn acquire(path: PathBuf) -> Result<Self> {
        match Self::try_acquire(path) {
            Ok(Some(lock)) => Ok(lock),
            Ok(None) => bail!("another instance is already running, please stop it first"),
            Err(err) => Err(err),
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.path) {
            warn!("Remove global lock file failed: {e:#}");
        }
    }
}
