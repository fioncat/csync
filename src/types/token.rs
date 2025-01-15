use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub user: String,
    pub token: String,
    pub expire_in: usize,
}
