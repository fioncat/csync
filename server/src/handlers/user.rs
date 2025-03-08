use chrono::Utc;
use csync_misc::api::metadata::GetMetadataRequest;
use csync_misc::api::user::{
    DeleteUserRequest, GetUserRequest, PatchUserRequest, PutUserRequest, User,
};
use csync_misc::api::{ListResponse, Response};
use csync_misc::code;
use log::{debug, error};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::context::ServerContext;
use crate::db::types::CreateUserParams;
use crate::register_handlers;

register_handlers!(put_user, get_user, patch_user, delete_user);

async fn put_user(mut req: PutUserRequest, op: User, sc: &ServerContext) -> Response<()> {
    if !op.admin {
        return Response::forbidden();
    }
    debug!("Create user: {req:?}");

    let result = sc.db.with_transaction(|tx| {
        if tx.has_user(req.name.clone())? {
            return Ok(false);
        }

        let salt = generate_salt(sc.cfg.salt_length);
        req.password = code::sha256(format!("{}{}", req.password, salt));

        let now = Utc::now().timestamp() as u64;

        tx.create_user(CreateUserParams {
            user: req,
            salt,
            update_time: now,
        })?;
        Ok(true)
    });

    match result {
        Ok(true) => Response::ok(),
        Ok(false) => Response::bad_request("user already exists"),
        Err(e) => {
            error!("Failed to create user: {e:#}");
            Response::database_error()
        }
    }
}

async fn get_user(
    req: GetUserRequest,
    op: User,
    sc: &ServerContext,
) -> Response<ListResponse<User>> {
    if !op.admin {
        match req.name {
            Some(ref name) => {
                if name != &op.name {
                    return Response::forbidden();
                }
            }
            None => {
                return Response::forbidden();
            }
        }
    }
    debug!("Get users: {req:?}");

    let result = sc.db.with_transaction(|tx| {
        let total = tx.count_users(req.clone())?;
        let users = tx.get_users(req)?;
        Ok(ListResponse {
            total,
            items: users,
        })
    });

    match result {
        Ok(users) => Response::with_data(users),
        Err(e) => {
            error!("Failed to get users: {e:#}");
            Response::database_error()
        }
    }
}

async fn patch_user(req: PatchUserRequest, op: User, sc: &ServerContext) -> Response<()> {
    if !op.admin && req.name != op.name {
        return Response::forbidden();
    }
    if !op.admin && req.admin.is_some() {
        return Response::forbidden();
    }

    debug!("Patch user: {req:?}");

    let result = sc.db.with_transaction(|tx| {
        if !tx.has_user(req.name.clone())? {
            return Ok(false);
        }

        let now = Utc::now().timestamp() as u64;
        tx.update_user(req, now)?;
        Ok(true)
    });

    match result {
        Ok(true) => Response::ok(),
        Ok(false) => Response::resource_not_found(),
        Err(e) => {
            error!("Failed to patch user: {e:#}");
            Response::database_error()
        }
    }
}

async fn delete_user(req: DeleteUserRequest, op: User, sc: &ServerContext) -> Response<()> {
    if !op.admin && req.name != op.name {
        return Response::forbidden();
    }

    debug!("Delete user: {req:?}");

    let result = sc.db.with_transaction(|tx| {
        if !tx.has_user(req.name.clone())? {
            return Ok(false);
        }
        tx.delete_user(&req.name)?;

        let blobs: Vec<_> = tx
            .get_metadatas(GetMetadataRequest {
                owner: Some(req.name.clone()),
                ..Default::default()
            })?
            .into_iter()
            .map(|m| m.id)
            .collect();

        if !blobs.is_empty() {
            debug!("Delete blobs belong to user {}: {:?}", req.name, blobs);
            tx.delete_blobs(blobs)?;
        }

        Ok(true)
    });

    match result {
        Ok(true) => Response::ok(),
        Ok(false) => Response::resource_not_found(),
        Err(e) => {
            error!("Failed to delete user: {e:#}");
            Response::database_error()
        }
    }
}

fn generate_salt(length: usize) -> String {
    let mut rng = thread_rng();

    (0..length)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}
