mod exec;
mod macos;
mod wayland;
mod x11;

use anyhow::{bail, Context, Result};
use std::env;

/// Represents different clipboard implementations based on platform
#[derive(Debug, Clone, Copy)]
pub enum Clipboard {
    /// macOS clipboard implementation
    Macos,
    /// Wayland clipboard implementation
    Wayland,
    /// X11 clipboard implementation
    X11,
}

impl Clipboard {
    /// Creates a new clipboard instance based on the current operating system and environment.
    ///
    /// # Returns
    /// - `Ok(Clipboard)` if a supported clipboard implementation is found and initialized
    /// - `Err` if the OS is not supported or clipboard initialization fails
    pub fn load() -> Result<Self> {
        let cb = match env::consts::OS {
            "linux" => {
                if env::var("WAYLAND_DISPLAY").is_ok() {
                    Clipboard::Wayland
                } else {
                    Clipboard::X11
                }
            }
            "macos" => Clipboard::Macos,
            _ => bail!("unsupported os {}", env::consts::OS),
        };
        cb.check().context("check clipboard")?;
        Ok(cb)
    }

    /// Checks if the clipboard implementation is available and working
    ///
    /// # Returns
    /// - `Ok(())` if the clipboard is available
    /// - `Err` if the clipboard is not available or not working
    fn check(&self) -> Result<()> {
        match self {
            Clipboard::Macos => macos::check(),
            Clipboard::Wayland => wayland::check(),
            Clipboard::X11 => x11::check(),
        }
    }

    /// Reads text content from the clipboard
    ///
    /// # Returns
    /// - `Ok(Some(String))` if text content is found
    /// - `Ok(None)` if no text content is available
    /// - `Err` if reading fails
    pub fn read_text(&self) -> Result<Option<String>> {
        match self {
            Clipboard::Macos => macos::read_text(),
            Clipboard::Wayland => wayland::read_text(),
            Clipboard::X11 => x11::read_text(),
        }
    }

    /// Writes text content to the clipboard
    ///
    /// # Arguments
    /// * `text` - The text to write to clipboard
    ///
    /// # Returns
    /// - `Ok(())` if write succeeds
    /// - `Err` if write fails
    pub fn write_text(&self, text: String) -> Result<()> {
        match self {
            Clipboard::Macos => macos::write_text(text),
            Clipboard::Wayland => wayland::write_text(text),
            Clipboard::X11 => x11::write_text(text),
        }
    }

    /// Reads image data from the clipboard
    ///
    /// # Returns
    /// - `Ok(Some(Vec<u8>))` if image data is found
    /// - `Ok(None)` if no image data is available
    /// - `Err` if reading fails
    pub fn read_image(&self) -> Result<Option<Vec<u8>>> {
        match self {
            Clipboard::Macos => macos::read_image(),
            Clipboard::Wayland => wayland::read_image(),
            Clipboard::X11 => x11::read_image(),
        }
    }

    /// Writes image data to the clipboard
    ///
    /// # Arguments
    /// * `data` - The image data to write to clipboard
    ///
    /// # Returns
    /// - `Ok(())` if write succeeds
    /// - `Err` if write fails
    pub fn write_image(&self, data: Vec<u8>) -> Result<()> {
        match self {
            Clipboard::Macos => macos::write_image(data),
            Clipboard::Wayland => wayland::write_image(data),
            Clipboard::X11 => x11::write_image(data),
        }
    }

    /// Checks if the clipboard contains image data
    ///
    /// # Returns
    /// - `Ok(true)` if clipboard contains image data
    /// - `Ok(false)` if clipboard does not contain image data
    /// - `Err` if check fails
    pub fn is_image(&self) -> Result<bool> {
        match self {
            Clipboard::Macos => macos::is_image(),
            Clipboard::Wayland => wayland::is_image(),
            Clipboard::X11 => x11::is_image(),
        }
    }

    /// Checks if the clipboard contains text data
    ///
    /// # Returns
    /// - `Ok(true)` if clipboard contains text data
    /// - `Ok(false)` if clipboard does not contain text data
    /// - `Err` if check fails
    pub fn is_text(&self) -> Result<bool> {
        match self {
            Clipboard::Macos => macos::is_text(),
            Clipboard::Wayland => wayland::is_text(),
            Clipboard::X11 => x11::is_text(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn should_run_tests() -> bool {
        std::env::var("TEST_CLIPBOARD").is_ok()
    }

    /// Note: Only runs when TEST_CLIPBOARD environment variable is set
    #[test]
    fn test_text() {
        if !should_run_tests() {
            println!("Skipping clipboard text test (TEST_CLIPBOARD not set)");
            return;
        }

        let cb = Clipboard::load().unwrap();
        cb.write_text("test".to_string()).unwrap();
        let text = cb.read_text().unwrap().unwrap();
        assert_eq!(text, "test");
        assert!(cb.is_text().unwrap());
    }

    /// Note: Only runs when TEST_CLIPBOARD environment variable is set
    #[test]
    fn test_image() {
        if !should_run_tests() {
            println!("Skipping clipboard image test (TEST_CLIPBOARD not set)");
            return;
        }

        let cb = Clipboard::load().unwrap();
        let image_data = include_bytes!("testdata/test.png").to_vec();
        cb.write_image(image_data.clone()).unwrap();

        if matches!(cb, Clipboard::Macos) {
            // macOS clipboard does not support image reading
            return;
        }

        let image = cb.read_image().unwrap().unwrap();
        assert_eq!(image, image_data);
        assert!(cb.is_image().unwrap());
    }
}
