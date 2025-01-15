use std::fs;
use std::path::Path;

use anyhow::Result;

/// Ensures that a directory exists at the specified path, creating it if necessary.
///
/// This function checks if a directory exists at the given path. If the directory
/// doesn't exist, it creates the directory and any necessary parent directories.
///
/// # Arguments
///
/// * `path` - The path where the directory should exist
///
/// # Returns
///
/// * `Ok(())` - Directory exists or was created successfully
/// * `Err(_)` - Failed to create directory (e.g., insufficient permissions)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use crate::dirs::ensure_dir_exists;
///
/// // Create a single directory
/// let dir_path = Path::new("data");
/// ensure_dir_exists(dir_path).unwrap();
///
/// // Create nested directories
/// let nested_path = Path::new("data/cache/temp");
/// ensure_dir_exists(nested_path).unwrap();
///
/// // Directory already exists - no error
/// ensure_dir_exists(dir_path).unwrap();
/// ```
pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_ensure_dir_exists() {
        // Create base test directory
        let base_path = Path::new("_test_ensure_dir");
        fs::create_dir_all(base_path).unwrap();

        // Test case 1: Create a new directory
        let new_dir = base_path.join("_test_dir");
        ensure_dir_exists(&new_dir).unwrap();
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());

        // Test case 2: Create nested directories
        let nested_dir = base_path.join("parent/child/grandchild");
        ensure_dir_exists(&nested_dir).unwrap();
        assert!(nested_dir.exists());
        assert!(nested_dir.is_dir());

        // Test case 3: Ensure existing directory doesn't cause error
        ensure_dir_exists(&new_dir).unwrap();
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());

        // Cleanup: Remove test directory and all its contents
        fs::remove_dir_all(base_path).unwrap();
    }
}
