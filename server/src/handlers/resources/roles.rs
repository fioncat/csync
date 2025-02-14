use std::sync::Arc;

use anyhow::Result;
use csync_misc::types::request::{PatchResource, Query};
use csync_misc::types::user::Role;
use log::error;

use crate::authn::AuthnUserInfo;
use crate::db::{Database, RoleRecord};
use crate::expect_json;
use crate::response::{self, Response};

use super::{PutRequest, ResourceHandler};

pub struct RolesHandler {
    db: Arc<Database>,
}

impl RolesHandler {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl ResourceHandler for RolesHandler {
    fn put(&self, req: PutRequest, _user: AuthnUserInfo) -> Response {
        let req_role: Role = expect_json!(req);

        if req_role.name.is_empty() {
            return Response::bad_request("Role name is required");
        }
        if req_role.rules.is_empty() {
            return Response::bad_request("Role rules are required");
        }
        for rule in req_role.rules.iter() {
            if rule.resources.is_empty() {
                return Response::bad_request("Rule resources are required");
            }
            if rule.verbs.is_empty() {
                return Response::bad_request("Rule verbs are required");
            }
        }

        let result: Result<()> = self.db.with_transaction(|tx, _cache| {
            if tx.is_role_exists(&req_role.name)? {
                tx.update_role_rules(&req_role.name, &req_role.rules)?;
            } else {
                let record = RoleRecord {
                    name: req_role.name.clone(),
                    rules: req_role.rules.clone(),
                    create_time: 0,
                    update_time: 0,
                };
                tx.create_role(&record)?;
            }
            Ok(())
        });

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Put role database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn patch(&self, _id: u64, _patch: PatchResource, _user: AuthnUserInfo) -> Response {
        Response::method_not_allowed()
    }

    fn list(&self, _query: Query, _json: bool, _user: AuthnUserInfo) -> Response {
        let records = match self.db.with_transaction(|tx, _cache| tx.list_roles()) {
            Ok(records) => records,
            Err(err) => {
                error!("List roles database error: {err:#}");
                return Response::error(response::DATABASE_ERROR);
            }
        };

        let mut roles = Vec::with_capacity(records.len());
        for record in records {
            roles.push(Role {
                name: record.name,
                rules: record.rules,
                create_time: record.create_time,
                update_time: record.update_time,
            });
        }

        Response::json(roles)
    }

    fn get(&self, id: String, _json: bool, _user: AuthnUserInfo) -> Response {
        let name = id;

        let result: Result<Option<Role>> = self.db.with_transaction(|tx, _cache| {
            if !tx.is_role_exists(&name)? {
                return Ok(None);
            }
            let record = tx.get_role(&name)?;
            let role = Role {
                name: record.name,
                rules: record.rules,
                create_time: record.create_time,
                update_time: record.update_time,
            };
            Ok(Some(role))
        });

        match result {
            Ok(role) => match role {
                Some(role) => Response::json(role),
                None => Response::not_found(),
            },
            Err(err) => {
                error!("Get role database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn delete(&self, id: String, _user: AuthnUserInfo) -> Response {
        let name = id;

        let mut not_found = false;
        let mut in_use = false;
        let result: Result<()> = self.db.with_transaction(|tx, _cache| {
            if !tx.is_role_exists(&name)? {
                not_found = true;
                return Ok(());
            }
            if tx.is_role_in_use(&name)? {
                in_use = true;
                return Ok(());
            }
            tx.delete_role(&name)?;
            Ok(())
        });

        if not_found {
            return Response::not_found();
        }

        if in_use {
            return Response::bad_request("Role is in use, cannot be deleted");
        }

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Delete role database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }
}
