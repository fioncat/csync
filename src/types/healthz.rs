use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthzResponse {
    pub now: u64,
    pub time_zone: String,
    pub client_ip: Option<String>,
    pub version: Option<String>,
}
