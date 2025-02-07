use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// MIME type for JSON content
pub const MIME_JSON: &str = "application/json";
/// MIME type for binary content
pub const MIME_OCTET_STREAM: &str = "application/octet-stream";

/// Common response structure for all API calls that don't return data
///
/// All API responses will contain at least a status code and optionally an error message.
/// A successful response typically has code 200 and no message.
#[derive(Serialize, Deserialize)]
pub struct CommonResponse {
    /// HTTP status code indicating the result of the operation
    /// - 200: Success
    /// - 400: Bad Request
    /// - 401: Unauthorized
    /// - 403: Forbidden
    /// - 404: Not Found
    /// - 500: Internal Server Error
    pub code: u16,

    /// Optional message providing additional information about the response,
    /// typically used to provide error details when the operation fails
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Generic response structure for API calls that return data
///
/// This extends CommonResponse to include a data field of generic type T.
/// The type T must implement both Serialize and DeserializeOwned traits
/// to support JSON serialization/deserialization.
#[derive(Serialize, Deserialize)]
#[serde(bound = "T: Serialize + DeserializeOwned")]
pub struct ResourceResponse<T: Serialize + DeserializeOwned> {
    /// HTTP status code indicating the result of the operation
    /// Same status codes as CommonResponse
    pub code: u16,

    /// Optional message providing additional information about the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// The actual response data of type T
    /// - Some(T): Contains the requested data on success
    /// - None: No data available or operation failed
    pub data: Option<T>,
}
