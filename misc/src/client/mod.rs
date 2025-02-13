pub mod config;
pub mod factory;
pub mod token;

use std::{fs, io};

use anyhow::{bail, Context, Result};
use chrono::Local;
use reqwest::{Certificate, Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::imghdr::is_data_image;
use crate::secret::aes::AesSecret;
use crate::secret::{base64_decode, Secret};
use crate::types::file::FileInfo;
use crate::types::healthz::HealthzResponse;
use crate::types::image::{Image, ENABLE_SECRET};
use crate::types::request::{Payload, Query};
use crate::types::response::{CommonResponse, ResourceResponse, MIME_JSON, MIME_OCTET_STREAM};
use crate::types::revision::RevisionResponse;
use crate::types::text::Text;
use crate::types::token::TokenResponse;
use crate::types::user::{CaniResponse, Role, User, WhoamiResponse};

/// Error types that can occur during client requests
#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Network error: {0}")]
    Network(#[from] anyhow::Error),

    #[error("Client error: {0}")]
    Client(String),

    #[error("Server error: code {code}, {message}")]
    Server { code: u16, message: String },

    #[error("Check health error: {0}")]
    Health(String),

    #[error("Unexpected error: {0}")]
    Unexpected(&'static str),

    #[error("Message require secret to decode, but you didn't provide one")]
    RequireSecret,

    #[error("Server returned invalid json: {0:?}")]
    InvalidJson(String),

    #[error("Your secret cannot decode messages from server, please check it")]
    InvalidSecret,

    #[error("Invalid image data, expect png or jpeg")]
    InvalidImage,

    #[error("Server returned inconsistent hash")]
    HashNotMatch,
}

/// A client for interacting with the server API
#[derive(Debug, Clone)]
pub struct Client {
    url: String,
    client: reqwest::Client,
    token: Option<String>,
    secret: Option<AesSecret>,
}

impl Client {
    /// Maximum allowed time difference between client and server in seconds
    pub const MAX_TIME_DELTA_WITH_SERVER: usize = 30;

    /// Creates a new client instance and connects to the server
    ///
    /// # Arguments
    /// * `url` - The server URL to connect to
    /// * `cert_path` - Path to SSL certificate file for HTTPS connections
    pub async fn connect(url: &str, cert_path: &str) -> Result<Self> {
        let url = url.trim_end_matches('/');
        let parsed = match Url::parse(url) {
            Ok(url) => url,
            Err(_) => bail!("invalid server url '{url}'"),
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

        let client = if cert_path.is_empty() || parsed.scheme() == "http" {
            reqwest::Client::new()
        } else {
            match fs::read(cert_path) {
                Ok(data) => {
                    let cert = Certificate::from_pem(&data).context("load cert file")?;
                    reqwest::Client::builder()
                        .add_root_certificate(cert)
                        // FIXME: Due to unknown reasons, reqwest's `add_root_certificate`
                        // call for self-signed certificates does not work properly.
                        // Therefore, we have to use `danger_accept_invalid_certs` to
                        // forcibly skip certificate validation. We need to wait for
                        //   <https://github.com/seanmonstar/reqwest/issues/1554>
                        // to be resolved before we can remove this call for self-signed
                        // certificates.
                        .danger_accept_invalid_certs(true)
                        .build()
                        .context("build server client")?
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => reqwest::Client::new(),
                Err(err) => return Err(err).context("read cert file"),
            }
        };

        let client = Client {
            url: url.to_string(),
            client,
            token: None,
            secret: None,
        };
        client.check_health().await?;

        Ok(client)
    }

    /// Sets the authentication token for subsequent requests
    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    /// Sets the encryption secret for secure data transfer
    pub fn set_secret(&mut self, secret: AesSecret) {
        self.secret = Some(secret);
    }

    /// Checks the server health status
    pub async fn healthz(&self) -> Result<HealthzResponse, RequestError> {
        self.do_request_data(Method::GET, "healthz", Payload::None, true)
            .await
    }

    /// Verifies server compatibility by checking timezone and time synchronization
    async fn check_health(&self) -> Result<(), RequestError> {
        let resp = self.healthz().await?;

        // Ensure server and client timezones match
        let now = Local::now();
        let time_zone = format!("{}", now.offset());
        if resp.time_zone != time_zone {
            return Err(RequestError::Health(format!(
                "timezone mismatch, server is '{}', client is '{}'",
                resp.time_zone, time_zone
            )));
        }

        // Time difference between server and client should not be too large
        // to avoid issues with time display and token expiration validation
        let now = Local::now().timestamp() as u64;
        let delta = if now > resp.now {
            now - resp.now
        } else {
            resp.now - now
        };
        if delta > Self::MAX_TIME_DELTA_WITH_SERVER as u64 {
            return Err(RequestError::Health(format!(
                "system time differs too much from server time: difference: {delta}s, maximum tolerance: {}s",
                Self::MAX_TIME_DELTA_WITH_SERVER
            )));
        }

        Ok(())
    }

    /// Login with username and password
    pub async fn login(&self, user: &str, password: &str) -> Result<TokenResponse, RequestError> {
        let path = format!("login/{user}");
        let resp: TokenResponse = self
            .do_request_data(
                Method::POST,
                &path,
                Payload::Binary(None, password.as_bytes().to_vec()),
                true,
            )
            .await?;
        Ok(resp)
    }

    /// Get current user name
    pub async fn whoami(&self) -> Result<String, RequestError> {
        let resp: WhoamiResponse = self
            .do_request_data(Method::GET, "api/whoami", Payload::None, true)
            .await?;
        Ok(resp.name)
    }

    /// Check if current user has permission to perform the action on the resource
    pub async fn cani(&self, verb: &str, resource: &str) -> Result<bool, RequestError> {
        let resp: CaniResponse = self
            .do_request_data(
                Method::GET,
                &format!("api/cani/{verb}/{resource}"),
                Payload::None,
                true,
            )
            .await?;
        Ok(resp.allow)
    }

    /// Get the server's current revision
    pub async fn revision(&self) -> Result<String, RequestError> {
        let resp: RevisionResponse = self
            .do_request_data(Method::GET, "api/revision", Payload::None, true)
            .await?;
        Ok(resp.revision)
    }

    /// Create or update a user
    pub async fn put_user(&self, user: &User) -> Result<(), RequestError> {
        let json = serde_json::to_string(user).unwrap();
        self.do_request_operation(Method::PUT, "api/users", Payload::Json(json))
            .await
    }

    /// Create or update a role
    pub async fn put_role(&self, role: &Role) -> Result<(), RequestError> {
        let json = serde_json::to_string(role).unwrap();
        self.do_request_operation(Method::PUT, "api/roles", Payload::Json(json))
            .await
    }

    /// Store text data on the server
    pub async fn put_text(&self, text: String) -> Result<Text, RequestError> {
        let hash = Sha256::digest(&text);
        let hash = format!("{:x}", hash);

        let data = match self.secret {
            Some(ref secret) => match secret.encrypt(text.as_bytes()) {
                Ok(data) => data,
                Err(_) => return Err(RequestError::InvalidSecret),
            },
            None => text.as_bytes().to_vec(),
        };

        let ret: Text = self
            .do_request_data(
                Method::PUT,
                "api/texts",
                Payload::Binary(Some(hash), data),
                true,
            )
            .await?;

        Ok(ret)
    }

    /// Query text records based on the provided query parameters
    pub async fn read_texts(&self, query: Query) -> Result<Vec<Text>, RequestError> {
        let query = serde_json::to_string(&query).unwrap();
        let mut texts: Vec<Text> = self
            .do_request_data(Method::GET, "api/texts", Payload::Json(query), false)
            .await?;

        for text in texts.iter_mut() {
            if text.secret {
                self.decrypt_text(text)?;
            }
            if text.content.is_none() {
                return Err(RequestError::Unexpected(
                    "text should be set when reading list",
                ));
            }

            let hash = Sha256::digest(text.content.as_ref().unwrap());
            let hash = format!("{:x}", hash);
            if hash != text.hash {
                return Err(RequestError::HashNotMatch);
            }
        }

        Ok(texts)
    }

    /// Read text data by ID
    pub async fn read_text(&self, id: u64) -> Result<Text, RequestError> {
        self._read_text(id.to_string()).await
    }

    /// Read the latest text data
    pub async fn read_latest_text(&self) -> Result<Text, RequestError> {
        self._read_text("latest".to_string()).await
    }

    async fn _read_text(&self, id: String) -> Result<Text, RequestError> {
        let path = format!("api/texts/{id}");

        let mut text: Text = self
            .do_request_data(Method::GET, &path, Payload::None, false)
            .await?;

        if text.secret {
            // If the text data is encrypted, we need to decrypt it first
            self.decrypt_text(&mut text)?;
        }

        // The text data returned by the server should match its hash
        let hash = Sha256::digest(text.content.as_ref().unwrap());
        let hash = format!("{:x}", hash);
        if hash != text.hash {
            return Err(RequestError::HashNotMatch);
        }

        Ok(text)
    }

    /// Decrypt text data using the client's secret key
    fn decrypt_text(&self, text: &mut Text) -> Result<(), RequestError> {
        if self.secret.is_none() {
            return Err(RequestError::RequireSecret);
        }
        let secret = self.secret.as_ref().unwrap();

        if text.content.is_none() {
            return Err(RequestError::Unexpected("text should be set when reading"));
        }

        // Text data is encrypted and then base64 encoded
        // So we need to decode first before decryption
        let encrypted = match base64_decode(text.content.as_ref().unwrap()) {
            Ok(encrypted) => encrypted,
            Err(_) => {
                return Err(RequestError::Unexpected(
                    "text should be base64 encoded when using secret",
                ))
            }
        };

        let data = match secret.decrypt(&encrypted) {
            Ok(content) => content,
            Err(_) => return Err(RequestError::InvalidSecret),
        };

        let content = match String::from_utf8(data) {
            Ok(content) => content,
            Err(_) => return Err(RequestError::InvalidSecret),
        };

        text.content = Some(content);
        Ok(())
    }

    /// Store image data on the server
    pub async fn put_image(&self, data: Vec<u8>) -> Result<Image, RequestError> {
        if !is_data_image(&data) {
            // Only allow valid image data
            return Err(RequestError::InvalidImage);
        }

        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);

        // If client has secret configured, encrypt the image data
        let data = match self.secret {
            Some(ref secret) => match secret.encrypt(&data) {
                Ok(data) => data,
                Err(_) => return Err(RequestError::InvalidSecret),
            },
            None => data,
        };

        let ret: Image = self
            .do_request_data(
                Method::PUT,
                "api/images",
                Payload::Binary(Some(hash), data),
                true,
            )
            .await?;
        Ok(ret)
    }

    /// Read image data by ID
    pub async fn read_image(&self, id: u64) -> Result<Vec<u8>, RequestError> {
        let path = format!("api/images/{id}");
        let (metadata, data) = self
            .do_request_binary(Method::GET, &path, Payload::None)
            .await?;
        let image_data = self.decode_image(metadata, data)?;
        Ok(image_data)
    }

    /// Read the latest image data
    pub async fn read_latest_image(&self) -> Result<Vec<u8>, RequestError> {
        let (metadata, data) = self
            .do_request_binary(Method::GET, "api/images/latest", Payload::None)
            .await?;
        let image_data = self.decode_image(metadata, data)?;
        Ok(image_data)
    }

    /// Decode image data, handling encryption if necessary
    fn decode_image(
        &self,
        metadata: Option<String>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>, RequestError> {
        // If metadata contains special flag, the image data is encrypted
        let mut is_secret = false;
        if let Some(metadata) = metadata {
            is_secret = metadata == ENABLE_SECRET;
        }

        let data = if is_secret {
            self.decrypt_data(data)?
        } else {
            data
        };

        // Image data must be valid PNG or JPEG format
        if !is_data_image(&data) {
            return Err(RequestError::InvalidImage);
        }

        Ok(data)
    }

    /// Store file data on the server
    pub async fn put_file(
        &self,
        name: String,
        mode: u32,
        data: Vec<u8>,
    ) -> Result<FileInfo, RequestError> {
        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);

        let mut info = FileInfo {
            id: 0,
            name,
            mode,
            hash,
            size: data.len() as u64,
            secret: false,
            owner: String::new(),
            create_time: 0,
        };

        let meta = serde_json::to_string(&info).unwrap();

        // If server has secret configured, encrypt the file data before uploading
        let data = match self.secret {
            Some(ref secret) => match secret.encrypt(&data) {
                Ok(data) => {
                    info.secret = true;
                    data
                }
                Err(_) => return Err(RequestError::InvalidSecret),
            },
            None => data,
        };

        let ret: FileInfo = self
            .do_request_data(
                Method::PUT,
                "api/files",
                Payload::Binary(Some(meta), data),
                true,
            )
            .await?;
        Ok(ret)
    }

    /// Read the latest file data and metadata
    pub async fn read_latest_file(&self) -> Result<(FileInfo, Vec<u8>), RequestError> {
        self._read_file("latest".to_string()).await
    }

    /// Read file data and metadata by ID
    pub async fn read_file(&self, id: u64) -> Result<(FileInfo, Vec<u8>), RequestError> {
        self._read_file(id.to_string()).await
    }

    async fn _read_file(&self, id: String) -> Result<(FileInfo, Vec<u8>), RequestError> {
        let path = format!("api/files/{id}");
        let (meta, mut data) = self
            .do_request_binary(Method::GET, &path, Payload::None)
            .await?;

        // File metadata is stored in the Metadata header as JSON
        let meta = match meta {
            Some(meta) => meta,
            None => return Err(RequestError::Unexpected("expect metadata for reading file")),
        };

        let info: FileInfo = match serde_json::from_str(&meta) {
            Ok(info) => info,
            Err(_) => return Err(RequestError::InvalidJson(meta)),
        };

        if info.secret {
            data = self.decrypt_data(data)?;
        }

        // The file's hash should match the hash returned by the server
        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);

        if info.hash != hash {
            return Err(RequestError::HashNotMatch);
        }

        Ok((info, data))
    }

    /// Decrypt data using the client's secret key
    fn decrypt_data(&self, data: Vec<u8>) -> Result<Vec<u8>, RequestError> {
        if self.secret.is_none() {
            return Err(RequestError::RequireSecret);
        }
        let secret = self.secret.as_ref().unwrap();

        let data = match secret.decrypt(&data) {
            Ok(content) => content,
            Err(_) => return Err(RequestError::InvalidSecret),
        };

        Ok(data)
    }

    /// Get a resource by name and ID
    pub async fn get_resource<T>(&self, name: &str, id: String) -> Result<T, RequestError>
    where
        T: Serialize + DeserializeOwned,
    {
        let path = format!("api/{name}/{id}");
        self.do_request_data(Method::GET, &path, Payload::None, true)
            .await
    }

    pub async fn get_resource_option<T>(
        &self,
        name: &str,
        id: String,
    ) -> Result<Option<T>, RequestError>
    where
        T: Serialize + DeserializeOwned,
    {
        let result = self.get_resource::<T>(name, id).await;
        match result {
            Ok(data) => Ok(Some(data)),
            Err(RequestError::Server { code, .. }) if code == StatusCode::NOT_FOUND => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// List resources by name with query parameters
    pub async fn list_resources<T>(&self, name: &str, query: Query) -> Result<Vec<T>, RequestError>
    where
        T: Serialize + DeserializeOwned,
    {
        let path = format!("api/{name}");
        let query = serde_json::to_string(&query).unwrap();
        self.do_request_data(Method::GET, &path, Payload::Json(query), true)
            .await
    }

    /// Delete a resource by name and ID
    pub async fn delete_resource(&self, name: &str, id: &str) -> Result<(), RequestError> {
        let path = format!("api/{name}/{id}");
        self.do_request_operation(Method::DELETE, &path, Payload::None)
            .await
    }

    /// Makes a request to the server and processes the response
    ///
    /// # Arguments
    /// * `method` - HTTP method to use
    /// * `path` - API endpoint path
    /// * `payload` - Request payload data
    /// * `with_accept` - Whether to include Accept header for JSON responses
    async fn do_request(
        &self,
        method: Method,
        path: &str,
        payload: Payload,
        with_accept: bool,
    ) -> Result<Payload, RequestError> {
        let url = format!("{}/{}", self.url, path);
        let mut req = self.client.request(method, &url);

        req = match payload {
            Payload::Json(json) => req.header("Content-Type", MIME_JSON).body(json),
            Payload::Binary(meta, data) => {
                let req = req.body(data).header("Content-Type", MIME_OCTET_STREAM);
                if let Some(metadata) = meta {
                    req.header("Metadata", metadata)
                } else {
                    req
                }
            }
            Payload::None => req,
        };

        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        if with_accept {
            // When getting resources, if Accept is set to json, it means the user needs
            // the resource metadata without the actual data. If Accept is not specified,
            // the server should return the complete resource data by default.
            req = req.header("Accept", "application/json");
        }

        let req = match req.build() {
            Ok(req) => req,
            Err(e) => return Err(RequestError::Client(format!("build request failed: {e:#}"))),
        };

        let resp = match self.client.execute(req).await {
            Ok(resp) => resp,
            Err(e) => return Err(RequestError::Network(e.into())),
        };

        // Server should always return Content-Type
        let ct = match resp.headers().get("Content-Type") {
            Some(ct) => ct,
            None => {
                return Err(RequestError::Unexpected(
                    "server didn't return content type header",
                ))
            }
        }
        .to_str()
        .ok()
        .unwrap_or_default();

        if ct.contains(MIME_JSON) {
            return resp
                .text()
                .await
                .map(Payload::Json)
                .map_err(|e| RequestError::Network(e.into()));
        }

        if ct.contains(MIME_OCTET_STREAM) {
            // For binary data returned by the server, additional metadata
            // may be recorded in the Metadata header.
            let meta = resp
                .headers()
                .get("Metadata")
                .and_then(|h| h.to_str().ok())
                .map(String::from);
            return resp
                .bytes()
                .await
                .map(|b| Payload::Binary(meta, b.to_vec()))
                .map_err(|e| RequestError::Network(e.into()));
        }

        Err(RequestError::Unexpected(
            "server returned unknown content type",
        ))
    }

    /// Makes a request expecting a JSON response and deserializes it to type T
    async fn do_request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        payload: Payload,
        with_accept: bool,
    ) -> Result<T, RequestError> {
        let resp = self.do_request(method, path, payload, with_accept).await?;

        let data = match resp {
            Payload::Json(json) => json,
            Payload::Binary(_, _) => {
                return Err(RequestError::Unexpected(
                    "server should return json, but it returned binary",
                ))
            }
            Payload::None => unreachable!(),
        };

        let data: T = match serde_json::from_str(&data) {
            Ok(data) => data,
            Err(_) => return Err(RequestError::InvalidJson(data)),
        };

        Ok(data)
    }

    /// Make a request that expects JSON data in response
    async fn do_request_data<T>(
        &self,
        method: Method,
        path: &str,
        payload: Payload,
        with_accept: bool,
    ) -> Result<T, RequestError>
    where
        T: Serialize + DeserializeOwned,
    {
        let resp: ResourceResponse<T> = self
            .do_request_json(method, path, payload, with_accept)
            .await?;
        if resp.code != StatusCode::OK {
            return Err(RequestError::Server {
                code: resp.code,
                message: resp.message.unwrap_or_default(),
            });
        }
        match resp.data {
            Some(data) => Ok(data),
            None => Err(RequestError::Unexpected(
                "server didn't return data in json",
            )),
        }
    }

    /// Make a request that expects a success/failure response
    async fn do_request_operation(
        &self,
        method: Method,
        path: &str,
        payload: Payload,
    ) -> Result<(), RequestError> {
        let resp: CommonResponse = self.do_request_json(method, path, payload, true).await?;

        if resp.code != StatusCode::OK {
            Err(RequestError::Server {
                code: resp.code,
                message: resp.message.unwrap_or_default(),
            })
        } else {
            Ok(())
        }
    }

    /// Make a request that expects binary data in response
    async fn do_request_binary(
        &self,
        method: Method,
        path: &str,
        payload: Payload,
    ) -> Result<(Option<String>, Vec<u8>), RequestError> {
        let resp = self.do_request(method, path, payload, false).await?;
        match resp {
            Payload::Binary(meta, data) => Ok((meta, data)),
            Payload::Json(json) => {
                let resp: CommonResponse = match serde_json::from_str(&json) {
                    Ok(data) => data,
                    Err(_) => return Err(RequestError::InvalidJson(json)),
                };

                if resp.code != StatusCode::OK {
                    return Err(RequestError::Server {
                        code: resp.code,
                        message: resp.message.unwrap_or_default(),
                    });
                }

                Err(RequestError::Unexpected(
                    "server should return binary, but it returned json",
                ))
            }
            Payload::None => unreachable!(),
        }
    }
}

impl RequestError {
    /// Checks if the error represents a "not found" response from the server
    pub fn is_not_found(&self) -> bool {
        matches!(self, RequestError::Server { code, .. } if *code == StatusCode::NOT_FOUND)
    }
}
