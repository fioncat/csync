use std::sync::Arc;

use actix_web::HttpRequest;
use csync_misc::secret::aes::AesSecret;
use csync_misc::types::request::{PatchResource, Payload, Query, ResourceRequest};
use csync_misc::types::revision::RevisionResponse;
use csync_misc::types::user::{CaniResponse, WhoamiResponse};
use log::error;

use crate::authn::chain::ChainAuthenticator;
use crate::authn::token::jwt::JwtTokenValidator;
use crate::authn::{Authenticator, AuthnResponse, AuthnUserInfo};
use crate::authz::chain::ChainAuthorizer;
use crate::authz::{Authorizer, AuthzRequest, AuthzResponse};
use crate::db::Database;
use crate::response::{self, Response};
use crate::revision::{Revisier, Revision};

use super::resources::dispatch::Dispatcher;
use super::Handler;

pub struct ApiHandler {
    authn: ChainAuthenticator<JwtTokenValidator>,
    authz: ChainAuthorizer,

    dispatcher: Dispatcher,

    revision: Arc<Revision>,
}

impl ApiHandler {
    pub fn new(
        authn: ChainAuthenticator<JwtTokenValidator>,
        authz: ChainAuthorizer,
        db: Arc<Database>,
        secret: Arc<Option<AesSecret>>,
        revision: Arc<Revision>,
    ) -> Self {
        Self {
            authn,
            authz,
            dispatcher: Dispatcher::new(db, secret, revision.clone()),
            revision,
        }
    }

    fn split_api_path(path: &str) -> Result<(String, Option<String>), &'static str> {
        // Remove trailing slash if present
        let path = path.trim_end_matches('/');

        // Split path into parts
        let parts: Vec<&str> = path.split('/').collect();

        match parts.as_slice() {
            [] => Err("empty path"),
            [""] => Err("empty resource"),
            [resource] => Ok((resource.to_string(), None)),
            [resource, id] => Ok((resource.to_string(), Some(id.to_string()))),
            _ => Err("invalid path format"),
        }
    }

    fn handle_whoami(&self, user: AuthnUserInfo) -> Response {
        Response::json(WhoamiResponse { name: user.name })
    }

    fn handle_cani(&self, path: &str, user: AuthnUserInfo) -> Response {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 2 {
            return Response::bad_request("Invalid path format");
        }

        let verb = parts[0];
        let resource = parts[1];

        match verb {
            "put" | "get" | "list" | "delete" => {}
            _ => return Response::bad_request("Invalid verb"),
        }

        let authz_req = AuthzRequest {
            resource: resource.to_string(),
            verb: verb.to_string(),
            user,
        };
        let authz_resp = match self.authz.authorize_request(&authz_req) {
            Ok(resp) => resp,
            Err(e) => {
                error!("Authorization for cani request failed: {e:#}");
                return Response::error(response::AUTHZ_ERROR);
            }
        };
        let allow = matches!(authz_resp, AuthzResponse::Ok);

        Response::json(CaniResponse { allow })
    }

    fn handle_revision(&self) -> Response {
        let rev = match self.revision.load() {
            Ok(rev) => rev,
            Err(e) => {
                error!("Load revision failed: {e:#}");
                return Response::error(response::REVISION_ERROR);
            }
        };

        Response::json(RevisionResponse { revision: rev })
    }
}

impl Handler for ApiHandler {
    fn handle(&self, path: &str, req: HttpRequest, body: Option<Vec<u8>>) -> Response {
        let method = req.method().as_str().to_lowercase();
        let authn_resp = match self.authn.authenticate_request(&req, None) {
            Ok(resp) => resp,
            Err(e) => {
                error!("Authentication failed: {e:#}");
                return Response::error(response::AUTHN_ERROR);
            }
        };
        let user_info = match authn_resp {
            AuthnResponse::Ok(user_info) => user_info,
            _ => return Response::unauthenticated("Invalid token"),
        };

        if path.starts_with("cani") {
            if method != "get" {
                return Response::method_not_allowed();
            }

            let path = path.strip_prefix("cani").unwrap();
            let path = path.trim_matches('/');

            return self.handle_cani(path, user_info);
        }

        let (resource, id) = match Self::split_api_path(path) {
            Ok((resource, id)) => (resource, id),
            Err(msg) => return Response::bad_request(msg),
        };

        if resource == "whoami" {
            if id.is_some() {
                return Response::bad_request("whoami does not take an id");
            }
            if method != "get" {
                return Response::method_not_allowed();
            }
            return self.handle_whoami(user_info);
        }

        if resource == "revision" {
            if id.is_some() {
                return Response::bad_request("revision does not take an id");
            }
            if method != "get" {
                return Response::method_not_allowed();
            }
            return self.handle_revision();
        }

        let content_type = req
            .headers()
            .get("Content-Type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let payload = match body {
            Some(data) if content_type.contains("application/json") => {
                match String::from_utf8(data) {
                    Ok(json_str) => Payload::Json(json_str),
                    Err(_) => {
                        return Response::bad_request("Invalid JSON encoding");
                    }
                }
            }
            Some(data) => {
                let metadata = req
                    .headers()
                    .get("Metadata")
                    .and_then(|h| h.to_str().ok())
                    .map(String::from);
                Payload::Binary(metadata, data)
            }
            None => Payload::None,
        };
        let accept = req
            .headers()
            .get("Accept")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let is_json = accept == "application/json";
        let rsc_req = match method.as_str() {
            "put" => match payload {
                Payload::Binary(metadata, data) => ResourceRequest::PutBinary(metadata, data),
                Payload::Json(json) => ResourceRequest::PutJson(json),
                Payload::None => {
                    return Response::bad_request("Request body is empty or too large")
                }
            },
            "patch" => match payload {
                Payload::Binary(_, _) => {
                    return Response::bad_request("Request body must be json for patch")
                }
                Payload::Json(json) => {
                    let patch: PatchResource = match serde_json::from_str(&json) {
                        Ok(patch) => patch,
                        Err(_) => return Response::bad_request("Invalid patch json"),
                    };
                    let id = match id {
                        Some(id) => id,
                        None => return Response::bad_request("Resource id is required"),
                    };
                    let id: u64 = match id.parse() {
                        Ok(id) => id,
                        Err(_) => return Response::bad_request("Invalid resource id"),
                    };

                    ResourceRequest::Patch(id, patch)
                }
                Payload::None => return Response::bad_request("Body is required"),
            },
            "get" => match id {
                Some(id) => ResourceRequest::Get(id, is_json),
                None => match payload {
                    Payload::Json(json) => {
                        let query: Query = match serde_json::from_str(&json) {
                            Ok(query) => query,
                            Err(_) => return Response::bad_request("Invalid query json"),
                        };
                        ResourceRequest::List(query, is_json)
                    }
                    Payload::None => ResourceRequest::List(Query::default(), is_json),
                    Payload::Binary(_, _) => {
                        return Response::bad_request("Request body must be json for query")
                    }
                },
            },
            "delete" => {
                let id = match id {
                    Some(id) => id,
                    None => return Response::bad_request("Resource id is required"),
                };
                ResourceRequest::Delete(id)
            }
            _ => return Response::method_not_allowed(),
        };

        let authz_req = AuthzRequest {
            resource: resource.clone(),
            verb: String::from(rsc_req.verb()),
            user: user_info,
        };
        let authz_resp = match self.authz.authorize_request(&authz_req) {
            Ok(resp) => resp,
            Err(e) => {
                error!("Authorization failed: {e:#}");
                return Response::error(response::AUTHZ_ERROR);
            }
        };
        match authz_resp {
            AuthzResponse::Ok => {}
            _ => return Response::unauthorized("Access denied"),
        };

        self.dispatcher.dispatch(rsc_req, &resource, authz_req.user)
    }
}
