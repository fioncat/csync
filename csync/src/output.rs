use chrono::Local;
use console::style;
use csync_proto::frame::{ClipboardFrame, DataFrame};

use crate::config::{Config, Target};

pub struct Output {
    publish: Option<String>,

    quiet_content: bool,
    quiet_all: bool,
}

impl Output {
    pub fn new(cfg: &Config, target: &Target) -> Output {
        Output {
            publish: target.publish.clone(),
            quiet_content: cfg.quiet_content,
            quiet_all: cfg.quiet_all,
        }
    }

    pub fn show(&self, frame: &DataFrame) {
        if self.quiet_all {
            return;
        }
        let now = Local::now();
        let datetime = now.format("%Y-%m-%d %H:%M:%S%.f");
        let datetime = style(datetime).cyan().bold();

        let belong = match frame.from.as_ref() {
            Some(from) => style(format!("<from: {from}>"))
                .magenta()
                .bold()
                .to_string(),
            None => match self.publish.as_ref() {
                Some(publish) => style(format!("<send: {publish}>"))
                    .green()
                    .bold()
                    .to_string(),
                None => style("<unknown>").red().bold().to_string(),
            },
        };

        let tail = match &frame.data {
            ClipboardFrame::Text(text) => style(format!("TEXT {}", human_bytes(text.len() as u64)))
                .yellow()
                .bold(),
            ClipboardFrame::Image(image) => style(format!(
                "IMAGE ({}, {}) {}",
                image.width,
                image.height,
                human_bytes(image.data.len() as u64)
            ))
            .blue()
            .bold(),
        };

        let line = format!("{datetime} {belong} {tail}");
        println!("{}", style(line).bold().cyan());

        if self.quiet_content {
            return;
        }

        if let ClipboardFrame::Text(text) = &frame.data {
            println!("{text}");
        }
        println!();
    }
}

/// Convert a size to a human-readable string, for example, "32KB".
pub fn human_bytes<T: Into<u64>>(bytes: T) -> String {
    const BYTES_UNIT: f64 = 1024.0;
    const BYTES_SUFFIX: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let size = bytes.into();
    let size = size as f64;
    if size <= 0.0 {
        return String::from("0B");
    }

    let base = size.log10() / BYTES_UNIT.log10();
    let result = format!("{:.1}", BYTES_UNIT.powf(base - base.floor()))
        .trim_end_matches(".0")
        .to_owned();

    [&result, BYTES_SUFFIX[base.floor() as usize]].join("")
}
