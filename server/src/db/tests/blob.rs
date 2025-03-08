use csync_misc::api::blob::{Blob, PatchBlobRequest};
use csync_misc::api::metadata::{BlobType, GetMetadataRequest, Metadata};
use csync_misc::api::QueryRequest;
use csync_misc::code;

use crate::db::types::{CreateBlobParams, PatchBlobParams};
use crate::db::Database;

pub fn run_blob_tests(db: &Database) {
    test_create(db);
    test_get(db);
    test_update(db);
    test_delete(db);
}

fn test_create(db: &Database) {
    let blobs = [
        CreateBlobParams {
            blob: Blob {
                data: "text".as_bytes().to_vec(),
                sha256: code::sha256("text".as_bytes()),
                blob_type: BlobType::Text,
                file_name: None,
                file_mode: None,
            },
            summary: String::from("a text"),
            owner: String::from("test_user"),
            update_time: 50,
            recycle_time: 100,
        },
        CreateBlobParams {
            blob: Blob {
                data: "image".as_bytes().to_vec(),
                sha256: code::sha256("image".as_bytes()),
                blob_type: BlobType::Image,
                file_name: None,
                file_mode: None,
            },
            summary: String::from("an image"),
            owner: String::from("test_user"),
            update_time: 60,
            recycle_time: 150,
        },
        CreateBlobParams {
            blob: Blob {
                data: "file".as_bytes().to_vec(),
                sha256: code::sha256("file".as_bytes()),
                blob_type: BlobType::File,
                file_name: Some(String::from("test_file")),
                file_mode: Some(123),
            },
            summary: String::from("a file"),
            owner: String::from("test_user2"),
            update_time: 30,
            recycle_time: 200,
        },
    ];

    db.with_transaction(|tx| {
        for blob in blobs {
            tx.create_blob(blob).unwrap();
        }
        Ok(())
    })
    .unwrap();
}

fn test_get(db: &Database) {
    let text_metadata = Metadata {
        id: 1,
        pin: false,
        blob_type: BlobType::Text,
        blob_sha256: code::sha256("text".as_bytes()),
        blob_size: "text".len() as u64,
        summary: String::from("a text"),
        owner: String::from("test_user"),
        update_time: 50,
        recycle_time: 100,
    };
    let text_blob = Blob {
        data: "text".as_bytes().to_vec(),
        sha256: code::sha256("text".as_bytes()),
        blob_type: BlobType::Text,
        file_name: None,
        file_mode: None,
    };
    let image_metadata = Metadata {
        id: 2,
        pin: false,
        blob_type: BlobType::Image,
        blob_sha256: code::sha256("image".as_bytes()),
        blob_size: "image".len() as u64,
        summary: String::from("an image"),
        owner: String::from("test_user"),
        update_time: 60,
        recycle_time: 150,
    };
    let image_blob = Blob {
        data: "image".as_bytes().to_vec(),
        sha256: code::sha256("image".as_bytes()),
        blob_type: BlobType::Image,
        file_name: None,
        file_mode: None,
    };
    let file_metadata = Metadata {
        id: 3,
        pin: false,
        blob_type: BlobType::File,
        blob_sha256: code::sha256("file".as_bytes()),
        blob_size: "file".len() as u64,
        summary: String::from("a file"),
        owner: String::from("test_user2"),
        update_time: 30,
        recycle_time: 200,
    };
    let file_blob = Blob {
        data: "file".as_bytes().to_vec(),
        sha256: code::sha256("file".as_bytes()),
        blob_type: BlobType::File,
        file_name: Some(String::from("test_file")),
        file_mode: Some(123),
    };
    db.with_transaction(|tx| {
        let metadatas = tx.get_metadatas(GetMetadataRequest::default())?;
        assert_eq!(metadatas.len(), 3);
        assert_eq!(metadatas[0], image_metadata);
        assert_eq!(metadatas[1], text_metadata);
        assert_eq!(metadatas[2], file_metadata);

        let metadatas = tx.get_metadatas(GetMetadataRequest {
            owner: Some(String::from("test_user")),
            query: QueryRequest {
                limit: Some(1),
                ..Default::default()
            },
            ..Default::default()
        })?;
        assert_eq!(metadatas.len(), 1);
        assert_eq!(metadatas[0], image_metadata);

        let metadatas = tx.get_metadatas(GetMetadataRequest {
            sha256: Some(code::sha256("file".as_bytes())),
            ..Default::default()
        })?;
        assert_eq!(metadatas.len(), 1);
        assert_eq!(metadatas[0], file_metadata);

        let metadatas = tx.get_metadatas(GetMetadataRequest {
            recycle_before: Some(160),
            ..Default::default()
        })?;
        assert_eq!(metadatas.len(), 2);
        assert_eq!(metadatas[0], image_metadata);
        assert_eq!(metadatas[1], text_metadata);

        let metadatas = tx.get_metadatas(GetMetadataRequest {
            query: QueryRequest {
                search: Some(String::from("im")),
                ..Default::default()
            },
            ..Default::default()
        })?;
        assert_eq!(metadatas.len(), 1);
        assert_eq!(metadatas[0], image_metadata);

        let metadatas = tx.get_metadatas(GetMetadataRequest {
            id: Some(1),
            ..Default::default()
        })?;
        assert_eq!(metadatas.len(), 1);
        assert_eq!(metadatas[0], text_metadata);

        let blob = tx.get_blob(1)?;
        assert_eq!(blob, text_blob);

        let blob = tx.get_blob(2)?;
        assert_eq!(blob, image_blob);

        let blob = tx.get_blob(3)?;
        assert_eq!(blob, file_blob);

        let count = tx.count_metadatas(GetMetadataRequest {
            owner: Some(String::from("test_user")),
            ..Default::default()
        })?;
        assert_eq!(count, 2);

        let count = tx.count_metadatas(GetMetadataRequest {
            id: Some(1),
            ..Default::default()
        })?;
        assert_eq!(count, 1);

        let count = tx.count_metadatas(GetMetadataRequest {
            sha256: Some(code::sha256("file".as_bytes())),
            ..Default::default()
        })?;
        assert_eq!(count, 1);

        let count = tx.count_metadatas(GetMetadataRequest {
            sha256: Some(code::sha256("empty".as_bytes())),
            ..Default::default()
        })?;
        assert_eq!(count, 0);

        let metadata = tx.get_metadata(1)?;
        assert_eq!(metadata, text_metadata);

        let metadata = tx.get_metadata(2)?;
        assert_eq!(metadata, image_metadata);

        let metadata = tx.get_metadata(3)?;
        assert_eq!(metadata, file_metadata);

        let result = tx.get_metadata(4);
        assert!(result.is_err());

        assert!(tx.has_blob(1)?);
        assert!(tx.has_blob(2)?);
        assert!(tx.has_blob(3)?);
        assert!(!tx.has_blob(4)?);

        Ok(())
    })
    .unwrap();
}

fn test_update(db: &Database) {
    let expect = Metadata {
        id: 2,
        pin: true,
        blob_type: BlobType::Image,
        blob_sha256: code::sha256("image".as_bytes()),
        blob_size: "image".len() as u64,
        summary: String::from("an image"),
        owner: String::from("test_user"),
        update_time: 2000,
        recycle_time: 0,
    };
    db.with_transaction(|tx| {
        tx.update_blob(PatchBlobParams {
            patch: PatchBlobRequest {
                id: 2,
                pin: Some(true),
            },
            update_time: 2000,
            recycle_time: 2030,
        })?;
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx| {
        let metadatas = tx.get_metadatas(GetMetadataRequest::default())?;
        assert_eq!(metadatas.len(), 3);
        assert_eq!(metadatas[0], expect);
        Ok(())
    })
    .unwrap();
}

fn test_delete(db: &Database) {
    db.with_transaction(|tx| {
        tx.delete_blob(1)?;
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx| {
        let metadatas = tx.get_metadatas(GetMetadataRequest::default())?;
        assert_eq!(metadatas.len(), 2);
        assert_eq!(metadatas[0].id, 2);
        assert_eq!(metadatas[1].id, 3);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx| {
        tx.delete_blobs(vec![2, 3])?;
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx| {
        let metadatas = tx.get_metadatas(GetMetadataRequest::default())?;
        assert_eq!(metadatas.len(), 0);

        let count = tx.count_metadatas(GetMetadataRequest::default())?;
        assert_eq!(count, 0);
        Ok(())
    })
    .unwrap();
}
