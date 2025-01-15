use std::collections::HashMap;
use std::sync::Arc;

use crate::secret::aes::AesSecret;
use crate::server::authn::AuthnUserInfo;
use crate::server::db::Database;
use crate::server::response::Response;
use crate::types::request::ResourceRequest;

use super::files::FilesHandler;
use super::images::ImagesHandler;
use super::roles::RolesHandler;
use super::texts::TextsHandler;
use super::union::UnionResourceHandler;
use super::users::UsersHandler;
use super::{PutRequest, ResourceHandler};

pub struct Dispatcher {
    handlers: HashMap<&'static str, Arc<UnionResourceHandler>>,
}

impl Dispatcher {
    pub fn new(db: Arc<Database>, secret: Arc<Option<AesSecret>>) -> Self {
        let mut handlers = HashMap::new();

        // users
        let handler = UsersHandler::new(db.clone());
        let handler = Arc::new(UnionResourceHandler::Users(handler));
        handlers.insert("users", handler.clone());

        // roles
        let handler = RolesHandler::new(db.clone());
        let handler = Arc::new(UnionResourceHandler::Roles(handler));
        handlers.insert("roles", handler.clone());

        // texts
        let handler = TextsHandler::new(db.clone(), secret.clone());
        let handler = Arc::new(UnionResourceHandler::Texts(handler));
        handlers.insert("texts", handler.clone());

        // images
        let handler = ImagesHandler::new(db.clone(), secret.clone());
        let handler = Arc::new(UnionResourceHandler::Images(handler));
        handlers.insert("images", handler.clone());

        // files
        let handler = FilesHandler::new(db.clone(), secret.clone());
        let handler = Arc::new(UnionResourceHandler::Files(handler));
        handlers.insert("files", handler.clone());

        Self { handlers }
    }

    pub fn dispatch(
        &self,
        rsc_req: ResourceRequest,
        resource: &str,
        user: AuthnUserInfo,
    ) -> Response {
        let handler = match self.handlers.get(resource) {
            Some(handler) => handler,
            None => return Response::not_found(),
        };

        match rsc_req {
            ResourceRequest::PutJson(s) => handler.put(PutRequest::Json(s), user),
            ResourceRequest::PutBinary(metadata, data) => {
                handler.put(PutRequest::Binary(metadata, data), user)
            }
            ResourceRequest::List(mut query, json) => {
                user.set_query_owner(&mut query);
                handler.list(query, json, user)
            }
            ResourceRequest::Get(id, json) => handler.get(id, json, user),
            ResourceRequest::Delete(id) => handler.delete(id, user),
        }
    }
}
