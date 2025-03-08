use std::collections::HashMap;

use actix_web::web::Bytes;
use actix_web::HttpRequest;
use anyhow::{bail, Context, Result};
use csync_misc::api::Request;
use csync_misc::header::HeaderMap;
use log::debug;
use url::form_urlencoded;

#[macro_export]
macro_rules! parse_request {
    ($req:expr, $body:expr) => {
        match $crate::request::parse_request_raw(&$req, $body) {
            Ok(user) => user,
            Err(e) => return csync_misc::api::Response::bad_request(format!("bad request: {e:#}")),
        }
    };
}

pub fn parse_request_raw<T>(req: &HttpRequest, body: Option<Bytes>) -> Result<T>
where
    T: Request,
{
    let query_string = req.query_string();

    let fields: HashMap<String, String> = form_urlencoded::parse(query_string.as_bytes())
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    debug!(
        "- {} {}, fields: {:?}, peer: {:?}, with_body: {:?}",
        req.method(),
        req.path(),
        fields,
        req.peer_addr(),
        body.is_some()
    );

    let mut parsed = T::default();
    parsed.complete(fields).context("parse query")?;

    let mut headers = HeaderMap::new();
    for (key, value) in req.headers() {
        let value = match value.to_str() {
            Ok(value) => value.to_string(),
            Err(_) => continue,
        };
        headers.insert(key.as_str(), value);
    }

    parsed.complete_headers(headers).context("parse headers")?;

    if parsed.is_data() {
        let body = body.map(|b| b.to_vec());
        match body {
            Some(data) => parsed.set_data(data),
            None => bail!("data is required"),
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use actix_web::test::TestRequest;
    use csync_misc::api::blob::{self, Blob};
    use csync_misc::api::metadata::{BlobType, GetMetadataRequest};
    use csync_misc::api::{QueryRequest, Response};

    use super::*;

    fn test_handler<T>(
        req: HttpRequest,
        body: Option<Bytes>,
        expect_request: Option<T>,
    ) -> Response<()>
    where
        T: Request + PartialEq + Debug,
    {
        let parsed: T = parse_request!(req, body);
        assert_eq!(parsed, expect_request.unwrap());
        Response::ok()
    }

    fn test_request<T>(
        query: Vec<(&str, &str)>,
        headers: Vec<(&str, &str)>,
        body: Option<&str>,
        expect_request: Option<T>,
    ) where
        T: Request + PartialEq + Debug,
    {
        let body = body.map(|s| s.as_bytes().to_vec()).map(Bytes::from);
        let mut url = String::from("http://127.0.0.1/api");
        if !query.is_empty() {
            url.push('?');
            for (i, (key, value)) in query.iter().enumerate() {
                if i > 0 {
                    url.push('&');
                }
                url.push_str(key);
                url.push('=');
                url.push_str(value);
            }
        }

        let mut req = TestRequest::with_uri(&url);
        for (key, value) in headers {
            req = req.insert_header((key, value));
        }

        let expect_err = expect_request.is_none();
        let resp = test_handler(req.to_http_request(), body, expect_request);

        if expect_err {
            assert_eq!(resp.code, 400);
            return;
        }

        assert_eq!(resp.code, 200);
    }

    #[test]
    fn test_parse_request() {
        test_request(
            vec![
                ("owner", "test"),
                ("sha256", "test_sha256"),
                ("limit", "20"),
            ],
            vec![],
            None,
            Some(GetMetadataRequest {
                owner: Some("test".to_string()),
                sha256: Some("test_sha256".to_string()),
                query: QueryRequest {
                    limit: Some(20),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );

        test_request(
            vec![("id", "123")],
            vec![],
            None,
            Some(GetMetadataRequest {
                id: Some(123),
                ..Default::default()
            }),
        );

        test_request(
            vec![],
            vec![
                (blob::HEADER_SHA256, "test_sha256"),
                (blob::HEADER_BLOB_TYPE, "text"),
            ],
            Some("test data"),
            Some(Blob {
                data: b"test data".to_vec(),
                sha256: "test_sha256".to_string(),
                blob_type: BlobType::Text,
                ..Default::default()
            }),
        );
        test_request(
            vec![],
            vec![
                (blob::HEADER_SHA256, "test_sha256"),
                (blob::HEADER_BLOB_TYPE, "image"),
            ],
            Some("test image"),
            Some(Blob {
                data: b"test image".to_vec(),
                sha256: "test_sha256".to_string(),
                blob_type: BlobType::Image,
                ..Default::default()
            }),
        );
        test_request(
            vec![],
            vec![
                (blob::HEADER_SHA256, "test_file_sha256"),
                (blob::HEADER_BLOB_TYPE, "file"),
                (blob::HEADER_FILE_NAME, "test_file"),
                (blob::HEADER_FILE_MODE, "12345"),
            ],
            Some("test file"),
            Some(Blob {
                data: b"test file".to_vec(),
                sha256: "test_file_sha256".to_string(),
                blob_type: BlobType::File,
                file_name: Some("test_file".to_string()),
                file_mode: Some(12345),
            }),
        );
        test_request(vec![], vec![], Some("test"), None::<Blob>);
        test_request(
            vec![],
            vec![(blob::HEADER_BLOB_TYPE, "file")],
            Some("test"),
            None::<Blob>,
        );
        test_request(
            vec![],
            vec![(blob::HEADER_BLOB_TYPE, "none")],
            Some("test"),
            None::<Blob>,
        );
    }
}
