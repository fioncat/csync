use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const MIME_JSON: &str = "application/json";
pub const MIME_OCTET_STREAM: &str = "application/octet-stream";

pub enum RawResponse {
    Json(String),
    Binary(Option<String>, Vec<u8>),
}

#[derive(Serialize, Deserialize)]
pub struct CommonResponse {
    pub code: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct ResourceResponse<T: Serialize + DeserializeOwned> {
    pub code: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    pub data: Option<T>,
}
