use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use log::{debug, info};
use reqwest::{Method, Url};
use serde::{de::DeserializeOwned, Serialize};

use crate::api::blob::{Blob, GetBlobRequest, PatchBlobRequest};
use crate::api::metadata::{GetMetadataRequest, Metadata, Revision};
use crate::api::user::{
    DeleteUserRequest, GetUserRequest, PatchUserRequest, PutUserRequest, TokenResponse, User,
};
use crate::api::{self, EmptyRequest, HealthResponse, ListResponse, Request, Response};
use crate::code;
use crate::header::HeaderMap;

pub struct RestfulClient {
    url: String,

    client: reqwest::Client,

    use_token: bool,
    basic_auth: String,
    token: Option<TokenResponse>,

    server_version: Option<String>,
}

pub struct RestfulClientBuilder {
    url: String,
    basic_auth: String,

    accept_invalid_certs: bool,

    use_token: bool,
}

impl RestfulClient {
    const MINIMAL_TIME_DIFF_SECS: u64 = 120;

    pub fn get_server_version(&self) -> &str {
        self.server_version.as_deref().unwrap_or("Unknown")
    }

    pub async fn put_blob(&mut self, blob: Blob) -> Result<()> {
        self.refresh_token().await?;
        let _: Response<()> = self
            .do_request(Method::PUT, api::blob::BLOB_PATH, blob)
            .await?;
        Ok(())
    }

    pub async fn get_blob(&mut self, id: u64) -> Result<Blob> {
        self.refresh_token().await?;
        let resp: Response<()> = self
            .do_request(Method::GET, api::blob::BLOB_PATH, GetBlobRequest { id })
            .await?;
        match resp.blob {
            Some(blob) => Ok(blob),
            None => bail!("missing blob in response"),
        }
    }

    pub async fn patch_blob(&mut self, patch: PatchBlobRequest) -> Result<()> {
        self.refresh_token().await?;
        let _: Response<()> = self
            .do_request(Method::PATCH, api::blob::BLOB_PATH, patch)
            .await?;
        Ok(())
    }

    pub async fn delete_blob(&mut self, id: u64) -> Result<()> {
        self.refresh_token().await?;
        let _: Response<()> = self
            .do_request(Method::DELETE, api::blob::BLOB_PATH, GetBlobRequest { id })
            .await?;
        Ok(())
    }

    pub async fn get_metadatas(
        &mut self,
        req: GetMetadataRequest,
    ) -> Result<ListResponse<Metadata>> {
        self.refresh_token().await?;
        let resp: Response<ListResponse<Metadata>> = self
            .do_request(Method::GET, api::metadata::METADATA_PATH, req)
            .await?;
        match resp.data {
            Some(data) => Ok(data),
            None => bail!("missing metadata list in response"),
        }
    }

    pub async fn get_metadata(&mut self, id: u64) -> Result<Metadata> {
        let req = GetMetadataRequest {
            id: Some(id),
            ..Default::default()
        };
        let mut list = self.get_metadatas(req).await?;
        if list.items.is_empty() {
            bail!("metadata not found: {}", id);
        }

        if list.items.len() > 1 {
            bail!("multiple metadata found: {}", id);
        }

        Ok(list.items.remove(0))
    }

    pub async fn get_revision(&mut self) -> Result<Revision> {
        self.refresh_token().await?;
        let resp: Response<Revision> = self
            .do_request(Method::GET, api::metadata::REVISION_PATH, EmptyRequest)
            .await?;
        Ok(resp.data.unwrap_or_default())
    }

    pub async fn put_user(&mut self, user: PutUserRequest) -> Result<()> {
        self.refresh_token().await?;
        let _: Response<()> = self
            .do_request(Method::PUT, api::user::USER_PATH, user)
            .await?;
        Ok(())
    }

    pub async fn get_users(&mut self, req: GetUserRequest) -> Result<ListResponse<User>> {
        self.refresh_token().await?;
        let resp: Response<ListResponse<User>> = self
            .do_request(Method::GET, api::user::USER_PATH, req)
            .await?;
        match resp.data {
            Some(data) => Ok(data),
            None => bail!("missing user list in response"),
        }
    }

    pub async fn get_user(&mut self, name: String) -> Result<User> {
        let req = GetUserRequest {
            name: Some(name.clone()),
            ..Default::default()
        };
        let mut list = self.get_users(req).await?;
        if list.items.is_empty() {
            bail!("user not found: {}", name);
        }
        if list.items.len() > 1 {
            bail!("multiple users found: {}", name);
        }
        Ok(list.items.remove(0))
    }

    pub async fn patch_user(&mut self, patch: PatchUserRequest) -> Result<()> {
        self.refresh_token().await?;
        let _: Response<()> = self
            .do_request(Method::PATCH, api::user::USER_PATH, patch)
            .await?;
        Ok(())
    }

    pub async fn delete_user(&mut self, name: String) -> Result<()> {
        self.refresh_token().await?;
        let _: Response<()> = self
            .do_request(
                Method::DELETE,
                api::user::USER_PATH,
                DeleteUserRequest { name },
            )
            .await?;
        Ok(())
    }

    async fn check_health(&mut self) -> Result<()> {
        self.refresh_token().await?;

        let resp: Response<HealthResponse> = self
            .do_request(Method::GET, api::HEALTHZ_PATH, EmptyRequest)
            .await?;

        let health = match resp.data {
            Some(health) => health,
            None => bail!("missing health in response"),
        };

        let now = Utc::now().timestamp() as u64;
        let delta = if now > health.timestamp {
            now - health.timestamp
        } else {
            health.timestamp - now
        };
        if delta > Self::MINIMAL_TIME_DIFF_SECS {
            bail!("time diff too large from server, please check your system clock");
        }

        info!(
            "Check server health done, server version: {}",
            health.version
        );
        self.server_version = Some(health.version);

        Ok(())
    }

    async fn refresh_token(&mut self) -> Result<()> {
        if !self.use_token {
            return Ok(());
        }

        let need_refresh = match self.token {
            Some(ref token) => {
                let now = Utc::now().timestamp() as u64;
                now > token.expire_after
            }
            None => true,
        };

        if !need_refresh {
            return Ok(());
        }

        info!("Token expired or not exist, refreshing token");
        self.token = None;
        let resp: Response<TokenResponse> = self
            .do_request(Method::GET, api::user::GET_TOKEN_PATH, EmptyRequest)
            .await?;
        let mut token = match resp.data {
            Some(token) => token,
            None => bail!("refresh: missing token in response"),
        };
        if token.token.is_empty() {
            bail!("refresh: empty token in response");
        }
        if token.expire_after == 0 {
            bail!("refresh: missing expire_after in response");
        }

        token.expire_after -= Self::MINIMAL_TIME_DIFF_SECS;
        info!("Token refreshed, expire_after: {}", token.expire_after);

        self.token = Some(token);
        Ok(())
    }

    async fn do_request<R, T>(&self, method: Method, path: &str, req: R) -> Result<Response<T>>
    where
        R: Request,
        T: Serialize + DeserializeOwned,
    {
        let url = format!("{}{}", self.url, path);

        let mut headers = HashMap::new();
        req.append_headers(&mut headers);
        if req.is_data() {
            headers.insert(api::HEADER_CONTENT_TYPE, api::MIME_OCTET_STREAM.to_string());
        }

        let mut req = if req.is_data() {
            let data = req.data();
            debug!(
                "Request server with data: {}, data_size: {}",
                url,
                data.len()
            );
            self.client.request(method, &url).body(data)
        } else {
            let fields = req.fields();
            let mut kvs = Vec::with_capacity(fields.len());
            for field in fields {
                let kv = format!("{}={}", field.name, field.value);
                kvs.push(kv);
            }
            let url = if kvs.is_empty() {
                url
            } else {
                format!("{}?{}", url, kvs.join("&"))
            };
            debug!("Request server with fields: {}", url);
            self.client.request(method, &url)
        };

        for (key, value) in headers {
            req = req.header(key, value);
        }

        if let Some(ref token) = self.token {
            req = req.header(api::HEADER_AUTHORIZATION, format!("Bearer {}", token.token));
        } else {
            req = req.header(
                api::HEADER_AUTHORIZATION,
                format!("Basic {}", self.basic_auth),
            );
        }

        let req = req.build().context("build restful request")?;

        let resp = self.client.execute(req).await?;

        let ct = match resp.headers().get(reqwest::header::CONTENT_TYPE) {
            Some(ct) => ct.to_str().context("parse content type")?,
            None => bail!("missing content type from response header"),
        };

        match ct {
            api::MIME_JSON => {
                let text = resp.text().await.context("read response text")?;
                let resp: Response<T> =
                    serde_json::from_str(&text).context("parse json response")?;
                if resp.code != 200 {
                    bail!(
                        "server error: {}, {}",
                        resp.code,
                        resp.message.unwrap_or_default()
                    );
                }

                Ok(resp)
            }
            api::MIME_OCTET_STREAM => {
                let headers = resp.headers().clone();
                let mut complete_headers = HeaderMap::new();
                for (key, value) in headers {
                    if let Some(key) = key {
                        complete_headers
                            .insert(key.as_str().to_string(), value.to_str()?.to_string());
                    }
                }
                let data = resp.bytes().await.context("read response bytes")?;

                let mut blob = Blob {
                    data: data.to_vec(),
                    ..Default::default()
                };
                blob.complete_headers(complete_headers)?;
                Ok(Response::with_blob(blob))
            }
            _ => bail!("unsupported content type '{}'", ct),
        }
    }
}

impl RestfulClientBuilder {
    pub fn new(url: &str, username: &str, password: &str) -> Self {
        let pwd_base64 = code::base64_encode(password);
        let basic_auth = format!("{}:{}", username, pwd_base64);

        Self {
            url: url.trim_end_matches('/').to_string(),
            basic_auth,
            accept_invalid_certs: false,
            use_token: false,
        }
    }

    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    pub fn use_token(mut self, use_token: bool) -> Self {
        self.use_token = use_token;
        self
    }

    pub async fn connect(self) -> Result<RestfulClient> {
        let parsed = match Url::parse(&self.url) {
            Ok(url) => url,
            Err(_) => bail!("invalid server url '{}'", self.url),
        };

        match parsed.scheme() {
            "http" | "https" => {}
            _ => bail!(
                "invalid url scheme, expect 'http' or 'https', not '{}'",
                parsed.scheme()
            ),
        }

        if parsed.path() != "/" {
            bail!(
                "invalid server url, path should be '/', not '{}'",
                parsed.path()
            );
        }

        let client = if self.accept_invalid_certs && parsed.scheme() == "https" {
            // FIXME: Due to unknown reasons, reqwest's `add_root_certificate`
            // call for self-signed certificates does not work properly.
            // Therefore, we have to use `danger_accept_invalid_certs` to
            // forcibly skip certificate validation. We need to wait for
            //   <https://github.com/seanmonstar/reqwest/issues/1554>
            // to be resolved before we can remove this call for self-signed
            // certificates.
            reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .context("build https client")?
        } else {
            reqwest::Client::new()
        };

        let mut client = RestfulClient {
            url: self.url,
            client,
            use_token: self.use_token,
            basic_auth: self.basic_auth,
            token: None,
            server_version: None,
        };
        client.check_health().await.context("check server health")?;

        Ok(client)
    }
}
