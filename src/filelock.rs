use std::io::{self, Read, Write};

use anyhow::Result;
use file_lock::FileLock;

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
    const ITERATIONS: usize = 100;
    const CONCURRENT_TASKS: usize = 100;

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
}
