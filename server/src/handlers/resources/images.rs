use std::sync::Arc;

use csync_misc::secret::aes::AesSecret;
use csync_misc::secret::Secret;
use csync_misc::types::image::{Image, ENABLE_SECRET};
use csync_misc::types::request::{PatchResource, Query};
use log::error;
use sha2::{Digest, Sha256};

use crate::authn::AuthnUserInfo;
use crate::db::{Database, ImageRecord};
use crate::expect_binary;
use crate::response::{self, Response};

use super::{PutRequest, ResourceHandler};

pub struct ImagesHandler {
    db: Arc<Database>,
    secret: Arc<Option<AesSecret>>,
}

impl ImagesHandler {
    pub fn new(db: Arc<Database>, secret: Arc<Option<AesSecret>>) -> Self {
        Self { db, secret }
    }
}

impl ResourceHandler for ImagesHandler {
    fn put(&self, req: PutRequest, user: AuthnUserInfo) -> Response {
        let (hash, mut data) = expect_binary!(req);
        if let Some(secret) = self.secret.as_ref() {
            data = match secret.decrypt(&data) {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to decrypt image data: {:#}", e);
                    return Response::bad_request("Invalid image secret");
                }
            };
        }

        let req_hash = match hash {
            Some(hash) => hash,
            None => {
                return Response::bad_request("Require image hash");
            }
        };

        let hash = Sha256::digest(&data);
        let hash = format!("{:x}", hash);

        if req_hash != hash {
            return Response::bad_request("Invalid image hash");
        }

        let size = data.len() as u64;
        let record = ImageRecord {
            id: 0,
            data,
            hash,
            size,
            pin: false,
            owner: user.name,
            create_time: 0,
        };

        let result = self.db.with_transaction(|tx, cache| {
            let hash_query = Query::new_hash(&record.owner, &record.hash);
            let duplicate_images = tx.list_images(hash_query)?;
            if !duplicate_images.is_empty() {
                let ids: Vec<_> = duplicate_images
                    .into_iter()
                    .map(|record| record.id)
                    .collect();
                tx.delete_images_batch(&ids)?;
            }

            let record = tx.create_image(record)?;
            cache.save_latest_image(&record.owner, record.clone())?;
            Ok(record)
        });

        match result {
            Ok(record) => {
                let image = self.convert_image(record);
                Response::json(image)
            }
            Err(e) => {
                error!("Failed to create image: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn patch(&self, id: u64, patch: PatchResource, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let result = if patch.pin {
            self.db.with_transaction(|tx, cache| {
                if !tx.is_image_exists(id, query_owner)? {
                    return Ok(false);
                }
                let image = tx.get_image(id, query_owner, true)?;
                let update = !image.pin;
                tx.update_image_pin(id, update)?;
                cache.clear_image()?;
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
                error!("Failed to patch image: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn list(&self, query: Query, _json: bool, _user: AuthnUserInfo) -> Response {
        let result = self.db.with_transaction(|tx, _cache| {
            let images = tx.list_images(query)?;
            Ok(images)
        });
        match result {
            Ok(records) => {
                let images: Vec<_> = records
                    .into_iter()
                    .map(|record| self.convert_image(record))
                    .collect();
                Response::json(images)
            }
            Err(e) => {
                error!("Failed to list images: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn get(&self, id: String, json: bool, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let record = if id == "latest" {
            let result = self.db.with_transaction(|tx, cache| {
                if let Some(image) = cache.get_latest_image(query_owner)? {
                    return Ok(Some(image));
                }
                if tx.count_images(query_owner, true)? == 0 {
                    return Ok(None);
                }
                let record = tx.get_latest_image(query_owner, json)?;
                cache.save_latest_image(&record.owner, record.clone())?;
                Ok(Some(record))
            });
            match result {
                Ok(Some(record)) => record,
                Ok(None) => return Response::not_found(),
                Err(e) => {
                    error!("Failed to get latest image: {:#}", e);
                    return Response::error(response::DATABASE_ERROR);
                }
            }
        } else {
            let id = match id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => return Response::bad_request("Invalid image id"),
            };
            if id == 0 {
                return Response::bad_request("Image id should not be zero");
            }

            let result = self.db.with_transaction(|tx, _cache| {
                if !tx.is_image_exists(id, query_owner)? {
                    return Ok(None);
                }
                let record = tx.get_image(id, query_owner, json)?;
                Ok(Some(record))
            });

            match result {
                Ok(Some(record)) => record,
                Ok(None) => return Response::not_found(),
                Err(e) => {
                    error!("Failed to get image: {:#}", e);
                    return Response::error(response::DATABASE_ERROR);
                }
            }
        };

        if json {
            return Response::json(self.convert_image(record));
        }

        if let Some(secret) = self.secret.as_ref() {
            let data = match secret.encrypt(&record.data) {
                Ok(encrypted) => encrypted,
                Err(e) => {
                    error!("Failed to encrypt image: {:#}", e);
                    return Response::error(response::SECRET_ERROR);
                }
            };
            return Response::binary(Some(String::from(ENABLE_SECRET)), data);
        }

        Response::binary(None, record.data)
    }

    fn delete(&self, id: String, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let id = match id.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return Response::bad_request("Invalid image id"),
        };

        let mut not_found = false;
        let result = self.db.with_transaction(|tx, cache| {
            if !tx.is_image_exists(id, query_owner)? {
                not_found = true;
                return Ok(());
            }
            cache.delete_latest_image(&user.name)?;
            tx.delete_image(id)?;
            Ok(())
        });

        if not_found {
            return Response::not_found();
        }

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Delete image database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }
}

impl ImagesHandler {
    fn convert_image(&self, record: ImageRecord) -> Image {
        Image {
            id: record.id,
            hash: record.hash,
            size: record.size,
            pin: record.pin,
            owner: record.owner,
            create_time: record.create_time,
        }
    }
}
