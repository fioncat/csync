use serde::{Deserialize, Serialize};

/// Health check information returned by the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthzResponse {
    /// Current server time
    pub now: u64,

    /// Server timezone, clients should be in the same timezone as the server
    pub time_zone: String,

    /// Client IP address obtained by the server, server may perform some blocking
    /// operations based on this IP
    pub client_ip: Option<String>,

    /// Server version
    pub version: Option<String>,
}
