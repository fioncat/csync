use std::fs;
use std::path::Path;

use anyhow::Result;

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    let path = Path::new(path);
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}
