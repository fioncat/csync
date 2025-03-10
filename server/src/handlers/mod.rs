use std::collections::HashMap;

use actix_web::HttpResponse;
use csync_misc::api::{self, Request, Response};
use serde::{de::DeserializeOwned, Serialize};

pub mod blob;
pub mod healthz;
pub mod metadata;
pub mod token;
pub mod user;

#[macro_export]
macro_rules! register_handlers {
    ($handler:ident) => {
        paste::paste! {
            pub async fn [< $handler _handler >](
                req: actix_web::HttpRequest,
                body: Option<actix_web::web::Bytes>,
                sc: actix_web::web::Data<std::sync::Arc<$crate::context::ServerContext>>,
            ) -> actix_web::HttpResponse {
                let f = || async move {
                    let user = $crate::auth_request!(sc.as_ref(), req);
                    let req = $crate::parse_request!(req, body);
                    $handler(req, user, sc.as_ref()).await
                };
                let resp = f().await;
                $crate::handlers::convert_response(resp)
            }
        }
    };

    ($handler:ident, $($rest:ident),* $(,)?) => {
        $crate::register_handlers!($handler);
        $crate::register_handlers!($($rest),*);
    };
}

pub fn convert_response<T>(resp: Response<T>) -> HttpResponse
where
    T: Serialize + DeserializeOwned,
{
    if let Some(blob) = resp.blob {
        let mut headers = HashMap::new();
        blob.append_headers(&mut headers);

        let mut http_resp = HttpResponse::Ok();
        for (key, value) in headers {
            http_resp.append_header((key, value));
        }

        http_resp.append_header((api::HEADER_CONTENT_TYPE, api::MIME_OCTET_STREAM));

        return http_resp.body(blob.data);
    }

    let mut http_resp = match resp.code {
        api::STATUS_OK => HttpResponse::Ok(),
        api::STATUS_BAD_REQUEST => HttpResponse::BadRequest(),
        api::STATUS_UNAUTHORIZED => HttpResponse::Unauthorized(),
        api::STATUS_FORBIDDEN => HttpResponse::Forbidden(),
        api::STATUS_NOT_FOUND => HttpResponse::NotFound(),
        _ => HttpResponse::InternalServerError(),
    };
    http_resp.json(resp)
}
