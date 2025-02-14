use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::TerminalDisplay;
use crate::humanize::human_bytes;
use crate::time::format_since;

/// This constant will be recorded in the GET request's Metadata Header, indicating that
/// the image data is encrypted and the receiving end needs to use the secret key for
/// decryption.
pub const ENABLE_SECRET: &str = "secret=true";

/// Image information stored on the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    /// Image ID
    pub id: u64,

    /// Image content hash using sha256
    pub hash: String,

    /// Image size
    pub size: u64,

    /// Image pin status
    pub pin: bool,

    /// Image owner
    pub owner: String,

    /// Image creation time
    pub create_time: u64,
}

impl TerminalDisplay for Image {
    fn table_titles() -> Vec<&'static str> {
        vec!["ID", "Size", "Owner", "Create"]
    }

    fn table_row(self) -> Vec<String> {
        let id = if self.pin {
            format!("* {}", self.id)
        } else {
            self.id.to_string()
        };
        vec![
            id,
            human_bytes(self.size),
            self.owner,
            format_since(self.create_time),
        ]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec!["id", "hash", "size", "pin", "owner", "create"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        vec![
            ("id", self.id.to_string()),
            ("hash", self.hash),
            ("size", self.size.to_string()),
            ("pin", self.pin.to_string()),
            ("owner", self.owner),
            ("create", self.create_time.to_string()),
        ]
        .into_iter()
        .collect()
    }
}
