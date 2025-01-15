mod exec;
mod macos;
mod wayland;
mod x11;

use anyhow::{bail, Context, Result};
use std::env;

#[derive(Debug, Clone, Copy)]
pub enum Clipboard {
    Macos,
    Wayland,
    X11,
}

impl Clipboard {
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

    fn check(&self) -> Result<()> {
        match self {
            Clipboard::Macos => macos::check(),
            Clipboard::Wayland => wayland::check(),
            Clipboard::X11 => x11::check(),
        }
    }

    pub fn read_text(&self) -> Result<Option<String>> {
        match self {
            Clipboard::Macos => macos::read_text(),
            Clipboard::Wayland => wayland::read_text(),
            Clipboard::X11 => x11::read_text(),
        }
    }

    pub fn write_text(&self, text: String) -> Result<()> {
        match self {
            Clipboard::Macos => macos::write_text(text),
            Clipboard::Wayland => wayland::write_text(text),
            Clipboard::X11 => x11::write_text(text),
        }
    }

    pub fn read_image(&self) -> Result<Option<Vec<u8>>> {
        match self {
            Clipboard::Macos => macos::read_image(),
            Clipboard::Wayland => wayland::read_image(),
            Clipboard::X11 => x11::read_image(),
        }
    }

    pub fn write_image(&self, data: Vec<u8>) -> Result<()> {
        match self {
            Clipboard::Macos => macos::write_image(data),
            Clipboard::Wayland => wayland::write_image(data),
            Clipboard::X11 => x11::write_image(data),
        }
    }

    pub fn is_image(&self) -> Result<bool> {
        match self {
            Clipboard::Macos => macos::is_image(),
            Clipboard::Wayland => wayland::is_image(),
            Clipboard::X11 => x11::is_image(),
        }
    }

    pub fn is_text(&self) -> Result<bool> {
        match self {
            Clipboard::Macos => macos::is_text(),
            Clipboard::Wayland => wayland::is_text(),
            Clipboard::X11 => x11::is_text(),
        }
    }
}
