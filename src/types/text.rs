use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::TerminalDisplay;
use crate::humanize::human_bytes;
use crate::time::format_since;

/// Text stored on the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    /// Text ID
    pub id: u64,

    /// Text content, may be None when only metadata is requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Text content hash using sha256
    pub hash: String,

    /// Text size in bytes
    pub size: u64,

    /// Text owner
    pub owner: String,

    /// Text creation time
    pub create_time: u64,

    /// Whether the text content is encrypted
    pub secret: bool,
}

impl TerminalDisplay for Text {
    fn table_titles() -> Vec<&'static str> {
        vec!["ID", "Size", "Owner", "Create"]
    }

    fn table_row(self) -> Vec<String> {
        vec![
            self.id.to_string(),
            human_bytes(self.size),
            self.owner,
            format_since(self.create_time),
        ]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec!["id", "size", "owner", "create"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        vec![
            ("id", self.id.to_string()),
            ("size", self.size.to_string()),
            ("owner", self.owner),
            ("create", self.create_time.to_string()),
        ]
        .into_iter()
        .collect()
    }
}
