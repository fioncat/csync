#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::FileLock;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::FileLock;
