use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::TerminalDisplay;
use crate::humanize::human_bytes;
use crate::time::format_since;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: u64,

    pub name: String,

    pub hash: String,

    pub size: u64,

    pub mode: u32,

    pub owner: String,
    pub create_time: u64,

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
