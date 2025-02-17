use csync_misc::types::request::{PatchResource, Query};

use crate::authn::AuthnUserInfo;
use crate::response::Response;

use super::files::FilesHandler;
use super::images::ImagesHandler;
use super::roles::RolesHandler;
use super::texts::TextsHandler;
use super::users::UsersHandler;
use super::{PutRequest, ResourceHandler};

pub enum UnionResourceHandler {
    Files(FilesHandler),
    Images(ImagesHandler),
    Roles(RolesHandler),
    Texts(TextsHandler),
    Users(UsersHandler),
}

impl ResourceHandler for UnionResourceHandler {
    fn put(&self, req: PutRequest, user: AuthnUserInfo) -> Response {
        match self {
            UnionResourceHandler::Files(handler) => handler.put(req, user),
            UnionResourceHandler::Images(handler) => handler.put(req, user),
            UnionResourceHandler::Roles(handler) => handler.put(req, user),
            UnionResourceHandler::Texts(handler) => handler.put(req, user),
            UnionResourceHandler::Users(handler) => handler.put(req, user),
        }
    }

    fn patch(&self, id: u64, patch: PatchResource, user: AuthnUserInfo) -> Response {
        match self {
            UnionResourceHandler::Files(handler) => handler.patch(id, patch, user),
            UnionResourceHandler::Images(handler) => handler.patch(id, patch, user),
            UnionResourceHandler::Roles(handler) => handler.patch(id, patch, user),
            UnionResourceHandler::Texts(handler) => handler.patch(id, patch, user),
            UnionResourceHandler::Users(handler) => handler.patch(id, patch, user),
        }
    }

    fn list(&self, query: Query, json: bool, user: AuthnUserInfo) -> Response {
        match self {
            UnionResourceHandler::Files(handler) => handler.list(query, json, user),
            UnionResourceHandler::Images(handler) => handler.list(query, json, user),
            UnionResourceHandler::Roles(handler) => handler.list(query, json, user),
            UnionResourceHandler::Texts(handler) => handler.list(query, json, user),
            UnionResourceHandler::Users(handler) => handler.list(query, json, user),
        }
    }

    fn get(&self, id: String, json: bool, user: AuthnUserInfo) -> Response {
        match self {
            UnionResourceHandler::Files(handler) => handler.get(id, json, user),
            UnionResourceHandler::Images(handler) => handler.get(id, json, user),
            UnionResourceHandler::Roles(handler) => handler.get(id, json, user),
            UnionResourceHandler::Texts(handler) => handler.get(id, json, user),
            UnionResourceHandler::Users(handler) => handler.get(id, json, user),
        }
    }

    fn delete(&self, id: String, user: AuthnUserInfo) -> Response {
        match self {
            UnionResourceHandler::Files(handler) => handler.delete(id, user),
            UnionResourceHandler::Images(handler) => handler.delete(id, user),
            UnionResourceHandler::Roles(handler) => handler.delete(id, user),
            UnionResourceHandler::Texts(handler) => handler.delete(id, user),
            UnionResourceHandler::Users(handler) => handler.delete(id, user),
        }
    }
}
