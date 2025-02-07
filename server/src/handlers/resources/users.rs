use std::sync::Arc;

use anyhow::{bail, Result};
use csync_misc::types::request::Query;
use csync_misc::types::user::{Role, User};
use log::error;

use crate::authn::AuthnUserInfo;
use crate::db::{Database, UserRecord};
use crate::expect_json;
use crate::response::{self, Response};

use super::{PutRequest, ResourceHandler};

pub struct UsersHandler {
    db: Arc<Database>,
}

impl UsersHandler {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl ResourceHandler for UsersHandler {
    fn put(&self, req: PutRequest, _user: AuthnUserInfo) -> Response {
        let req_user: User = expect_json!(req);
        if req_user.name == "admin" {
            return Response::unauthorized("Cannot modify admin user");
        }

        if req_user.name.is_empty() {
            return Response::bad_request("User name is required");
        }
        for role in req_user.roles.iter() {
            if role.name.is_empty() {
                return Response::bad_request("Role name is required");
            }
        }
        if let Some(ref password) = req_user.password {
            if password.is_empty() {
                return Response::bad_request("Password cannot be empty");
            }
        }
        let hash = req_user.generate_password_hash();

        let mut bad_request = false;
        let result: Result<()> = self.db.with_transaction(|tx, cache| {
            let mut created = false;
            if !tx.is_user_exists(&req_user.name)? {
                let (hash, salt) = match hash {
                    Some((hash, salt)) => (hash, salt),
                    None => {
                        bad_request = true;
                        bail!("password is required for new user");
                    }
                };

                if req_user.roles.is_empty() {
                    bad_request = true;
                    bail!("roles is required for new user");
                }

                let record = UserRecord {
                    name: req_user.name.clone(),
                    hash,
                    salt,
                    create_time: 0,
                    update_time: 0,
                };
                tx.create_user(&record)?;
                created = true;
            } else if let Some((hash, salt)) = hash {
                tx.update_user_password(&req_user.name, &hash, &salt)?;
            }

            if !req_user.roles.is_empty() {
                cache.delete_user_roles(&req_user.name)?;
                if !created {
                    // Delete all existing roles for the user
                    tx.delete_user_roles(&req_user.name)?;
                }
                // Grant the user new roles
                for role in req_user.roles.iter() {
                    if !tx.is_role_exists(&role.name)? {
                        bad_request = true;
                        bail!("role '{}' does not exist", role.name);
                    }
                    tx.create_user_role(&req_user.name, &role.name)?;
                }
                if !created {
                    // Update the user's update time
                    tx.update_user_time(&req_user.name)?;
                }
            }

            Ok(())
        });

        if bad_request {
            let err = result.unwrap_err();
            return Response::bad_request(format!("{err:#}"));
        }

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Put user database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn list(&self, _query: Query, _json: bool, _user: AuthnUserInfo) -> Response {
        let records = match self.db.with_transaction(|tx, _cache| tx.list_users()) {
            Ok(records) => records,
            Err(err) => {
                error!("List users database error: {err:#}");
                return Response::error(response::DATABASE_ERROR);
            }
        };

        let mut users: Vec<User> = Vec::with_capacity(records.len());
        for record in records {
            let user = User {
                name: record.name,
                create_time: record.create_time,
                update_time: record.update_time,
                password: None,
                roles: Vec::new(),
            };
            users.push(user);
        }

        Response::json(users)
    }

    fn get(&self, id: String, _json: bool, _user: AuthnUserInfo) -> Response {
        let name = id;
        if name == "admin" {
            return Response::unauthorized("Cannot get admin user");
        }

        let result: Result<Option<User>> = self.db.with_transaction(|tx, _cache| {
            if !tx.is_user_exists(&name)? {
                return Ok(None);
            }
            let record = tx.get_user(&name)?;
            let roles: Vec<Role> = tx
                .list_user_roles(&name)?
                .into_iter()
                .map(|r| Role {
                    name: r.name,
                    rules: r.rules,
                    create_time: r.create_time,
                    update_time: r.update_time,
                })
                .collect();
            let user = User {
                name: record.name,
                create_time: record.create_time,
                update_time: record.update_time,
                password: None,
                roles,
            };
            Ok(Some(user))
        });

        match result {
            Ok(Some(user)) => Response::json(user),
            Ok(None) => Response::not_found(),
            Err(err) => {
                error!("Get user database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn delete(&self, id: String, _user: AuthnUserInfo) -> Response {
        let name = id;
        if name == "admin" {
            return Response::unauthorized("Cannot delete admin user");
        }

        let mut not_found = false;
        let result: Result<()> = self.db.with_transaction(|tx, cache| {
            if !tx.is_user_exists(&name)? {
                not_found = true;
                return Ok(());
            }
            cache.delete_user_roles(&name)?;
            tx.delete_user_roles(&name)?;
            tx.delete_user(&name)
        });

        if not_found {
            return Response::not_found();
        }

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Delete user database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }
}
