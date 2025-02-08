use std::sync::Arc;

use csync_misc::secret::aes::AesSecret;
use csync_misc::secret::{base64_encode, Secret};
use csync_misc::types::request::Query;
use csync_misc::types::text::Text;
use log::error;
use sha2::{Digest, Sha256};

use crate::authn::AuthnUserInfo;
use crate::db::{Database, TextRecord};
use crate::expect_binary;
use crate::response::{self, Response};

use super::{PutRequest, ResourceHandler};

pub struct TextsHandler {
    db: Arc<Database>,
    secret: Arc<Option<AesSecret>>,
}

impl TextsHandler {
    pub fn new(db: Arc<Database>, secret: Arc<Option<AesSecret>>) -> Self {
        Self { db, secret }
    }
}

impl ResourceHandler for TextsHandler {
    fn put(&self, req: PutRequest, user: AuthnUserInfo) -> Response {
        let (hash, mut data) = expect_binary!(req);
        if let Some(secret) = self.secret.as_ref() {
            data = match secret.decrypt(&data) {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to decrypt text data: {:#}", e);
                    return Response::bad_request("Invalid text secret");
                }
            };
        }

        let req_hash = match hash {
            Some(hash) => hash,
            None => {
                return Response::bad_request("Require text hash");
            }
        };

        let content = match String::from_utf8(data) {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to parse text utf8: {:#}", e);
                return Response::bad_request("Invalid text content");
            }
        };

        let hash = Sha256::digest(content.as_bytes());
        let hash = format!("{:x}", hash);

        if req_hash != hash {
            return Response::bad_request("Invalid text hash");
        }

        let size = content.len() as u64;
        let record = TextRecord {
            id: 0,
            content,
            hash,
            size,
            owner: user.name,
            create_time: 0,
        };

        let result = self.db.with_transaction(|tx, cache| {
            let hash_query = Query::new_hash(&record.owner, &record.hash);
            let duplicate_texts = tx.list_texts(hash_query, true)?;
            if !duplicate_texts.is_empty() {
                let ids: Vec<_> = duplicate_texts
                    .into_iter()
                    .map(|record| record.id)
                    .collect();
                tx.delete_texts_batch(&ids)?;
            }

            let record = tx.create_text(record)?;
            cache.save_latest_text(&record.owner, record.clone())?;
            Ok(record)
        });

        match result {
            Ok(record) => {
                let text = self.convert_text(record, true);
                Response::json(text)
            }
            Err(e) => {
                error!("Failed to create text: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn list(&self, query: Query, json: bool, _user: AuthnUserInfo) -> Response {
        let result = self.db.with_transaction(|tx, _cache| {
            let texts = tx.list_texts(query, json)?;
            Ok(texts)
        });
        match result {
            Ok(records) => {
                let mut texts: Vec<_> = records
                    .into_iter()
                    .map(|record| self.convert_text(record, json))
                    .collect();
                if json {
                    return Response::json(texts);
                }
                if let Some(secret) = self.secret.as_ref() {
                    for text in texts.iter_mut() {
                        let encrypted =
                            match secret.encrypt(text.content.as_ref().unwrap().as_bytes()) {
                                Ok(encrypted) => encrypted,
                                Err(e) => {
                                    error!("Failed to encrypt text: {:#}", e);
                                    return Response::error(response::SECRET_ERROR);
                                }
                            };
                        text.content = Some(base64_encode(&encrypted));
                        text.secret = true;
                    }
                }
                Response::json(texts)
            }
            Err(e) => {
                error!("Failed to list texts: {:#}", e);
                Response::error(response::DATABASE_ERROR)
            }
        }
    }

    fn get(&self, id: String, json: bool, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let mut text = if id == "latest" {
            let result = self.db.with_transaction(|tx, cache| {
                if let Some(text) = cache.get_latest_text(query_owner)? {
                    return Ok(Some(text));
                }
                if tx.count_texts(query_owner)? == 0 {
                    return Ok(None);
                }
                let record = tx.get_latest_text(query_owner, json)?;
                cache.save_latest_text(&record.owner, record.clone())?;
                Ok(Some(record))
            });
            match result {
                Ok(Some(record)) => self.convert_text(record, json),
                Ok(None) => return Response::not_found(),
                Err(e) => {
                    error!("Failed to get latest text: {:#}", e);
                    return Response::error(response::DATABASE_ERROR);
                }
            }
        } else {
            let id = match id.parse::<u64>() {
                Ok(id) => id,
                Err(_) => return Response::bad_request("Invalid text id"),
            };
            if id == 0 {
                return Response::bad_request("Text id should not be zero");
            }

            let result = self.db.with_transaction(|tx, _cache| {
                if !tx.is_text_exists(id, query_owner)? {
                    return Ok(None);
                }
                let record = tx.get_text(id, query_owner, json)?;
                Ok(Some(record))
            });

            match result {
                Ok(Some(record)) => self.convert_text(record, json),
                Ok(None) => return Response::not_found(),
                Err(e) => {
                    error!("Failed to get text: {:#}", e);
                    return Response::error(response::DATABASE_ERROR);
                }
            }
        };

        if json {
            return Response::json(text);
        }

        if let Some(secret) = self.secret.as_ref() {
            let encrypted = match secret.encrypt(text.content.unwrap().as_bytes()) {
                Ok(encrypted) => encrypted,
                Err(e) => {
                    error!("Failed to encrypt text: {:#}", e);
                    return Response::error(response::SECRET_ERROR);
                }
            };
            text.content = Some(base64_encode(&encrypted));
            text.secret = true;
        }

        Response::json(text)
    }

    fn delete(&self, id: String, user: AuthnUserInfo) -> Response {
        let query_owner = user.get_query_owner();
        let id = match id.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return Response::bad_request("Invalid text id"),
        };

        let mut not_found = false;
        let result = self.db.with_transaction(|tx, cache| {
            if !tx.is_text_exists(id, query_owner)? {
                not_found = true;
                return Ok(());
            }
            cache.delete_latest_text(&user.name)?;
            tx.delete_text(id)?;
            Ok(())
        });

        if not_found {
            return Response::not_found();
        }

        match result {
            Ok(()) => Response::ok(),
            Err(err) => {
                error!("Delete text database error: {err:#}");
                Response::error(response::DATABASE_ERROR)
            }
        }
    }
}

impl TextsHandler {
    fn convert_text(&self, record: TextRecord, json: bool) -> Text {
        if json {
            Text {
                id: record.id,
                content: None,
                hash: record.hash,
                size: record.size,
                owner: record.owner,
                create_time: record.create_time,
                secret: false,
            }
        } else {
            Text {
                id: record.id,
                content: Some(record.content),
                hash: record.hash,
                size: record.size,
                owner: record.owner,
                create_time: record.create_time,
                secret: false,
            }
        }
    }
}
