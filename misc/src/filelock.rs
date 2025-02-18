use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process;

use anyhow::{bail, Context, Result};
use file_lock::FileLock;
use log::warn;

/// A global file-based lock to ensure only one instance of a process is running.
///
/// This lock writes the current process ID to a file and maintains an exclusive lock
/// on that file until the `GlobalLock` instance is dropped.
///
/// # Examples
/// ```
/// use std::path::PathBuf;
/// use csync_misc::filelock::GlobalLock;
///
/// let lock_path = PathBuf::from("testdata/process.lock");
/// let lock = GlobalLock::acquire(lock_path).expect("Failed to acquire lock");
/// // Process is now locked
/// // Lock is automatically released when `lock` is dropped
/// ```
pub struct GlobalLock {
    path: PathBuf,
    /// Wrap the `file_lock` crate
    _file_lock: file_lock::FileLock,
}

impl GlobalLock {
    /// Error code returned by the OS when a file lock cannot be acquired
    const RESOURCE_TEMPORARILY_UNAVAILABLE_CODE: i32 = 11;

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
        Ok(Some(GlobalLock {
            path,
            _file_lock: file_lock,
        }))
    }

    pub fn acquire(path: PathBuf) -> Result<Self> {
        match Self::try_acquire(path) {
            Ok(Some(lock)) => Ok(lock),
            Ok(None) => bail!("another instance is already running, please stop it first"),
            Err(err) => Err(err),
        }
    }
}

impl Drop for GlobalLock {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.path) {
            warn!("Remove global lock file failed: {:#}", e);
        }
    }
}

/// Reads data from a file with shared lock.
///
/// # Arguments
/// * `path` - Path to the file to read
///
/// # Returns
/// * `Ok(Some(Vec<u8>))` - File contents if file exists and read succeeds
/// * `Ok(None)` - If file does not exist
/// * `Err` - If file operations fail
pub fn read_file_lock(path: &str) -> Result<Option<Vec<u8>>> {
    let lock_opts = file_lock::FileOptions::new().read(true);
    let mut file = match FileLock::lock(path, true, lock_opts) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let mut data = Vec::new();
    file.file.read_to_end(&mut data)?;
    Ok(Some(data))
}

/// Writes data to a file with exclusive lock.
///
/// Creates the file if it doesn't exist, truncates it if it does.
///
/// # Arguments
/// * `path` - Path to the file to write
/// * `data` - Data to write to the file
///
/// # Returns
/// * `Ok(())` - If write succeeds
/// * `Err` - If file operations fail
pub fn write_file_lock(path: &str, data: &[u8]) -> Result<()> {
    let lock_opts = file_lock::FileOptions::new()
        .write(true)
        .truncate(true)
        .create(true);
    let mut file = FileLock::lock(path, true, lock_opts)?;
    file.file.write_all(data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    const TEST_FILE: &str = "_test_filelock";
    const TEST_CONTENT: &[u8] = b"Hello, World!";
    const ITERATIONS: usize = 10;
    const CONCURRENT_TASKS: usize = 10;

    async fn concurrent_read_write() -> Result<()> {
        let tasks: Vec<_> = (0..CONCURRENT_TASKS)
            .map(|_| {
                tokio::spawn(async {
                    for _ in 0..ITERATIONS {
                        // Write test
                        write_file_lock(TEST_FILE, TEST_CONTENT)?;

                        // Read and verify test
                        let content = read_file_lock(TEST_FILE)?.expect("File should exist");
                        assert_eq!(content, TEST_CONTENT);
                    }
                    Ok::<_, anyhow::Error>(())
                })
            })
            .collect();

        for task in tasks {
            task.await.unwrap()?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_file_operations() {
        // Clean up any existing test file
        let _ = fs::remove_file(TEST_FILE);

        // Run concurrent operations
        concurrent_read_write().await.unwrap();

        // Verify final content
        let final_content = read_file_lock(TEST_FILE).unwrap().unwrap();
        assert_eq!(final_content, TEST_CONTENT);

        // Clean up
        fs::remove_file(TEST_FILE).unwrap();
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let result = read_file_lock("nonexistent_file").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_global_lock_basic() {
        let lock_path = PathBuf::from("_test_global.lock");

        // Clean up any existing lock file
        let _ = fs::remove_file(&lock_path);

        // Acquire lock
        let lock = GlobalLock::acquire(lock_path.clone()).unwrap();

        // Verify lock file exists and contains current PID
        let content = fs::read(&lock_path).unwrap();
        let pid_str = String::from_utf8(content).unwrap();
        assert_eq!(pid_str, process::id().to_string());

        // Drop lock
        drop(lock);

        // Verify lock file is removed after drop
        assert!(!lock_path.exists());
    }
}
