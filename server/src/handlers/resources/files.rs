use csync_misc::secret::aes::AesSecret;
use csync_misc::secret::Secret;
use csync_misc::types::file::FileInfo;
use csync_misc::types::request::{PatchResource, Query};
use log::error;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::authn::AuthnUserInfo;
use crate::db::{Database, FileRecord};
use crate::expect_binary;
use crate::handlers::resources::PutRequest;
use crate::response::{self, Response};

use super::ResourceHandler;

pub struct FilesHandler {
    db: Arc<Database>,
    secret: Arc<Option<AesSecret>>,
}

impl FilesHandler {
    pub fn new(db: Arc<Database>, secret: Arc<Option<AesSecret>>) -> Self {
        Self { db, secret }
    }
}

impl ResourceHandler for FilesHandler {
    fn put(&self, req: PutRequest, user: AuthnUserInfo) -> Response {
        let (json, mut data) = expect_binary!(req);
        if let Some(secret) = self.secret.as_ref() {
            data = match secret.decrypt(&data) {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to decrypt image data: {:#}", e);
                    return Response::bad_request("Invalid image secret");
                }
            };
        }

        let json = match json {
            Some(json) => json,
            None => return Response::bad_request("File info is required"),
        };

        let info: FileInfo = match serde_json::from_str(&json) {
            Ok(info) => info,
            Err(e) => {
                error!("Failed to decode file info: {:#}", e);
                return Response::bad_request("Invalid file info");
            }
        };

        if info.name.is_empty() {
            return Response::bad_request("File name cannot be empty");
        }
        if info.name == "latest" {
            return Response::bad_request("File name cannot be 'latest', please use another name");
        }

        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);
        if info.hash != hash {
            return Response::bad_request("Invalid file hash");
        }

        let size = data.len() as u64;
        let record = FileRecord {
            id: 0,
            name: info.name,
            data,
            hash,
            size,
            mode: info.mode,
            pin: false,
            owner: user.name,
            create_time: 0,
        };

        let result = self.db.with_transaction(|tx, cache| {
            let record = tx.create_file(record)?;
            cache.save_latest_file(&record.owner, record.clone())?;
            Ok(record)
        });

        match result {
            Ok(record) => {
                let file = self.convert_file(record);
                Response::json(file)
            }
            Err(e) => {
                error!("Failed to create or update file: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn patch(&self, id: u64, patch: PatchResource, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let result = if patch.pin {
            self.db.with_transaction(|tx, cache| {
                if !tx.is_file_exists(id, query_owner)? {
                    return Ok(false);
                }
                let file = tx.get_file(id, query_owner, true)?;
                let update = !file.pin;
                tx.update_file_pin(id, update)?;
                cache.clear_file()?;
                Ok(true)
            })
        } else {
            Ok(false)
        };

        match result {
            Ok(ok) => {
                if ok {
                    Response::ok()
                } else {
                    Response::not_found()
                }
            }
            Err(e) => {
                error!("Failed to patch file: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn list(&self, query: Query, _json: bool, _user: AuthnUserInfo) -> Response {
        let result = self.db.with_transaction(|tx, _cache| {
            let files = tx.list_files(query)?;
            Ok(files)
        });
        match result {
            Ok(records) => {
                let files: Vec<_> = records
                    .into_iter()
                    .map(|record| self.convert_file(record))
                    .collect();
                Response::json(files)
            }
            Err(e) => {
                error!("Failed to list files: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn get(&self, id: String, json: bool, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let record = if id == "latest" {
            let result = self.db.with_transaction(|tx, cache| {
                if let Some(file) = cache.get_latest_file(query_owner)? {
                    return Ok(Some(file));
                }
                if tx.count_files(query_owner, true)? == 0 {
                    return Ok(None);
                }
                let record = tx.get_latest_file(query_owner, json)?;
                cache.save_latest_file(&record.owner, record.clone())?;
                Ok(Some(record))
            });
            match result {
                Ok(Some(record)) => record,
                Ok(None) => return Response::not_found(),
                Err(e) => {
                    error!("Failed to get latest file: {:#}", e);
                    return Response::error(response::DATABASE_ERROR);
                }
            }
        } else {
            let id = match id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => return Response::bad_request("Invalid file id"),
            };
            if id == 0 {
                return Response::bad_request("File id should not be zero");
            }

            let result = self.db.with_transaction(|tx, _cache| {
                if !tx.is_file_exists(id, query_owner)? {
                    return Ok(None);
                }
                let record = tx.get_file(id, query_owner, json)?;
                Ok(Some(record))
            });

            match result {
                Ok(Some(record)) => record,
                Ok(None) => return Response::not_found(),
                Err(e) => {
                    error!("Failed to get file: {:#}", e);
                    return Response::error(response::DATABASE_ERROR);
                }
            }
        };

        if json {
            return Response::json(self.convert_file(record));
        }

        let FileRecord {
            id,
            name,
            mut data,
            hash,
            size,
            mode,
            pin,
            owner,
            create_time,
        } = record;

        let mut info = FileInfo {
            id,
            name,
            hash,
            size,
            mode,
            pin,
            owner,
            create_time,
            secret: false,
        };

        if let Some(secret) = self.secret.as_ref() {
            data = match secret.encrypt(&data) {
                Ok(encrypted) => encrypted,
                Err(e) => {
                    error!("Failed to encrypt image: {:#}", e);
                    return Response::error(response::SECRET_ERROR);
                }
            };
            info.secret = true;
        }

        let meta = match serde_json::to_string(&info) {
            Ok(meta) => meta,
            Err(e) => {
                error!("Failed to encode file info: {:#}", e);
                return Response::error(response::JSON_ERROR);
            }
        };

        Response::binary(Some(meta), data)
    }

    fn delete(&self, id: String, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let id = match id.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return Response::bad_request("Invalid file id"),
        };

        let mut not_found = false;
        let result = self.db.with_transaction(|tx, cache| {
            if !tx.is_file_exists(id, query_owner)? {
                not_found = true;
                return Ok(());
            }
            cache.delete_latest_file(&user.name)?;
            tx.delete_file(id)?;
            Ok(())
        });

        if not_found {
            return Response::not_found();
        }

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Delete file database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }
}

impl FilesHandler {
    fn convert_file(&self, record: FileRecord) -> FileInfo {
        FileInfo {
            id: record.id,
            name: record.name,
            hash: record.hash,
            size: record.size,
            mode: record.mode,
            pin: record.pin,
            owner: record.owner,
            create_time: record.create_time,
            secret: false,
        }
    }
}
