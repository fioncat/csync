use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::TerminalDisplay;
use crate::humanize::human_bytes;
use crate::time::format_since;

/// File information stored on the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File ID
    pub id: u64,

    /// File name
    pub name: String,

    /// File content hash using sha256
    pub hash: String,

    /// File size
    pub size: u64,

    /// File unix mode
    pub mode: u32,

    /// File owner
    pub owner: String,

    /// File creation time
    pub create_time: u64,

    /// Whether the file content is encrypted
    pub secret: bool,
}

impl TerminalDisplay for FileInfo {
    fn table_titles() -> Vec<&'static str> {
        vec!["ID", "Name", "Size", "Owner", "Create"]
    }

    fn table_row(self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.name,
            human_bytes(self.size),
            self.owner,
            format_since(self.create_time),
        ]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec!["id", "name", "hash", "size", "mode", "owner", "create"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        vec![
            ("id", self.id.to_string()),
            ("name", self.name),
            ("hash", self.hash),
            ("size", self.size.to_string()),
            ("mode", self.mode.to_string()),
            ("owner", self.owner),
            ("create", self.create_time.to_string()),
        ]
        .into_iter()
        .collect()
    }
}
