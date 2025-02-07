use serde::{Deserialize, Serialize};

/// Response structure for successful authentication
///
/// Contains the authentication token and related information returned by the server
/// after a successful login.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// Username associated with this token
    pub user: String,

    /// The authentication token string that should be used in subsequent requests
    pub token: String,

    /// Timestamp when this token will expire
    /// The timezone of this timestamp is specified in the HealthzResponse
    pub expire_in: usize,
}
