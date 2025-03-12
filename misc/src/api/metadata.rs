use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::api::Value;
use crate::display::TerminalDisplay;
use crate::{humanize, parse_from_map, time};

use super::{QueryRequest, Request, RequestField};

pub const METADATA_PATH: &str = "/v1/metadata";
pub const STATE_PATH: &str = "/v1/state";

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Metadata {
    pub id: u64,

    pub pin: bool,

    pub blob_type: BlobType,
    pub blob_sha256: String,
    pub blob_size: u64,

    pub summary: String,

    pub owner: String,

    pub update_time: u64,
    pub recycle_time: u64,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum BlobType {
    #[serde(rename = "text")]
    #[default]
    Text,

    #[serde(rename = "image")]
    Image,

    #[serde(rename = "file")]
    File,
}

impl TerminalDisplay for Metadata {
    fn table_titles() -> Vec<&'static str> {
        vec!["ID", "Type", "Size", "Owner", "Update", "Recycle"]
    }

    fn table_row(self) -> Vec<String> {
        let id = if self.pin {
            format!("* {}", self.id)
        } else {
            self.id.to_string()
        };
        let size = humanize::human_bytes(self.blob_size);
        let update_time = time::format_time(self.update_time);
        let recycle_time = time::format_time(self.recycle_time);

        vec![
            id,
            self.blob_type.to_string(),
            size,
            self.owner,
            update_time,
            recycle_time,
        ]
    }

    fn csv_titles() -> Vec<&'static str> {
        vec![
            "id",
            "pin",
            "type",
            "sha256",
            "size",
            "owner",
            "update_time",
            "recycle_time",
        ]
    }
    fn csv_row(self) -> HashMap<&'static str, String> {
        let mut row = HashMap::new();
        row.insert("id", self.id.to_string());
        row.insert("pin", self.pin.to_string());
        row.insert("type", self.blob_type.to_string());
        row.insert("sha256", self.blob_sha256);
        row.insert("size", self.blob_size.to_string());
        row.insert("owner", self.owner);
        row.insert("update_time", self.update_time.to_string());
        row.insert("recycle_time", self.recycle_time.to_string());
        row
    }
}

impl Display for BlobType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match self {
            BlobType::Text => "text",
            BlobType::Image => "image",
            BlobType::File => "file",
        };
        write!(f, "{s}")
    }
}

impl FromStr for BlobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(BlobType::Text),
            "image" => Ok(BlobType::Image),
            "file" => Ok(BlobType::File),
            _ => Err(format!("invalid blob type '{}'", s)),
        }
    }
}

impl BlobType {
    pub fn to_code(&self) -> u32 {
        match self {
            BlobType::Text => 1,
            BlobType::Image => 2,
            BlobType::File => 3,
        }
    }

    pub fn parse_code(code: u32) -> Result<BlobType> {
        match code {
            1 => Ok(BlobType::Text),
            2 => Ok(BlobType::Image),
            3 => Ok(BlobType::File),
            _ => bail!("invalid blob type code '{}'", code),
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct GetMetadataRequest {
    pub id: Option<u64>,

    pub owner: Option<String>,

    pub sha256: Option<String>,

    pub recycle_before: Option<u64>,

    pub query: QueryRequest,
}

impl Request for GetMetadataRequest {
    fn fields(self) -> Vec<RequestField> {
        if let Some(id) = self.id {
            return vec![RequestField {
                name: "id",
                value: Value::Integer(id),
            }];
        }

        let mut fields = Vec::new();
        if let Some(owner) = self.owner {
            fields.push(RequestField {
                name: "owner",
                value: Value::Text(owner),
            });
        }

        if let Some(sha256) = self.sha256 {
            fields.push(RequestField {
                name: "sha256",
                value: Value::Text(sha256),
            });
        }

        fields.extend(self.query.fields());

        fields
    }

    fn complete(&mut self, mut fields: HashMap<String, String>) -> Result<()> {
        if let Some(id) = parse_from_map!(fields, "id") {
            self.id = Some(id);
            return Ok(());
        }

        self.owner = fields.remove("owner");
        self.sha256 = fields.remove("sha256");

        self.query.complete(fields)?;

        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct ServerState {
    pub rev: Option<u64>,
    pub latest: Option<Metadata>,
}
