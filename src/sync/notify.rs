use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{bail, Context, Result};
use file_lock::FileLock;

pub fn read<P: AsRef<Path>>(path: P) -> Result<Option<Vec<u8>>> {
    let filelock = acquire_filelock(path.as_ref())?;

    let data = match fs::read(path.as_ref()) {
        Ok(data) => {
            fs::remove_file(path.as_ref())
                .with_context(|| format!("remove notify file '{}'", path.as_ref().display()))?;
            Some(data)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(err)
                .with_context(|| format!("read notify file '{}'", path.as_ref().display()))
        }
    };
    drop(filelock);

    Ok(data)
}

pub fn write<P: AsRef<Path>>(path: P, data: &[u8]) -> Result<()> {
    let filelock = acquire_filelock(path.as_ref())?;

    fs::write(path.as_ref(), data)
        .with_context(|| format!("write notify file '{}'", path.as_ref().display()))?;
    drop(filelock);

    Ok(())
}

fn acquire_filelock<P: AsRef<Path>>(path: P) -> Result<FileLock> {
    let path = PathBuf::from(path.as_ref());
    let dir = path.parent();
    if dir.is_none() {
        bail!("invalid notify path '{}', missing dir", path.display());
    }
    let lock_path = dir.unwrap().join("notify_lock");

    let opts = file_lock::FileOptions::new()
        .write(true)
        .create(true)
        .truncate(true);
    FileLock::lock(&lock_path, true, opts)
        .with_context(|| format!("acquire filelock '{}'", lock_path.display()))
}
