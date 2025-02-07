use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder};
use csync_misc::types::response::{CommonResponse, ResourceResponse, MIME_OCTET_STREAM};
use serde::{de::DeserializeOwned, Serialize};

pub const AUTHN_ERROR: &str = "Authentication failed";
pub const AUTHZ_ERROR: &str = "Authorization failed";
pub const DATABASE_ERROR: &str = "Database error";
pub const TOKEN_ERROR: &str = "Generate token failed";
pub const SECRET_ERROR: &str = "Generate or validate secret failed";
pub const JSON_ERROR: &str = "Encode or decode JSON failed";

/// A wrapper struct for HTTP responses that provides convenient methods
/// for creating common response types
pub struct Response {
    http_response: HttpResponse,
}

impl Response {
    pub fn not_found() -> Self {
        Self::err_response(StatusCode::NOT_FOUND, "Resource not found".to_string())
    }

    pub fn bad_request(message: impl AsRef<str>) -> Self {
        let message = format!("Bad request: {}", message.as_ref());
        Self::err_response(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthenticated(message: impl AsRef<str>) -> Self {
        let message = format!("Unauthenticated: {}", message.as_ref());
        Self::err_response(StatusCode::UNAUTHORIZED, message)
    }

    pub fn unauthorized(message: &str) -> Self {
        let message = format!("Unauthorized: {message}");
        Self::err_response(StatusCode::FORBIDDEN, message)
    }

    pub fn method_not_allowed() -> Self {
        Self::err_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "Method not allowed".to_string(),
        )
    }

    pub fn error(message: &str) -> Self {
        let message = format!("Server error: {message}");
        Self::err_response(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn ok() -> Self {
        Self::ok_response()
    }

    pub fn json<T: Serialize + DeserializeOwned>(data: T) -> Self {
        Self::resource_response(data)
    }

    pub fn binary(metadata: Option<String>, data: Vec<u8>) -> Self {
        let mut resp = HttpResponse::Ok();
        if let Some(metadata) = metadata {
            resp.append_header(("Metadata", metadata));
        }
        resp.append_header(("Content-Type", MIME_OCTET_STREAM));
        Self {
            http_response: resp.body(data),
        }
    }

    fn ok_response() -> Self {
        let resp = CommonResponse {
            code: StatusCode::OK.into(),
            message: None,
        };
        Self {
            http_response: HttpResponse::Ok().json(resp),
        }
    }

    fn resource_response<T: Serialize + DeserializeOwned>(rsc: T) -> Self {
        let resp = ResourceResponse::<T> {
            code: StatusCode::OK.into(),
            message: None,
            data: Some(rsc),
        };
        Self {
            http_response: HttpResponse::Ok().json(resp),
        }
    }

    fn err_response(status: StatusCode, message: String) -> Self {
        let resp = CommonResponse {
            code: status.into(),
            message: Some(message),
        };
        Self {
            http_response: HttpResponseBuilder::new(status).json(resp),
        }
    }
}

impl From<Response> for HttpResponse {
    fn from(val: Response) -> Self {
        val.http_response
    }
}
