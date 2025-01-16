use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::display::TerminalDisplay;
use crate::humanize::human_bytes;
use crate::time::format_since;

pub const ENABLE_SECRET: &str = "secret=true";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: u64,

    pub hash: String,

    pub size: u64,

    pub owner: String,
    pub create_time: u64,
}

impl TerminalDisplay for Image {
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
        vec!["id", "hash", "size", "owner", "create"]
    }

    fn csv_row(self) -> HashMap<&'static str, String> {
        vec![
            ("id", self.id.to_string()),
            ("hash", self.hash),
            ("size", self.size.to_string()),
            ("owner", self.owner),
            ("create", self.create_time.to_string()),
        ]
        .into_iter()
        .collect()
    }
}