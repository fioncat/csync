use sha2::{Digest, Sha256};

use crate::server::db::{Database, ImageRecord};
use crate::time::{advance_mock_time, current_timestamp};
use crate::types::request::Query;

pub fn run_image_tests(db: &Database) {
    let images = [
        mock_image("image01"),
        mock_image("image02"),
        mock_image("image03"),
        mock_image("image04"),
        mock_image("png"),
        mock_image("jpeg"),
        mock_image("test image"),
    ];

    let mut create_images = vec![];
    db.with_transaction(|tx, _cache| {
        for image in images.iter() {
            let ret = tx.create_image(image.clone()).unwrap();
            create_images.push(ret);
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(create_images.len(), images.len());
    db.with_transaction(|tx, _cache| {
        for image in create_images.iter() {
            let ret = tx.get_image(image.id, None, true).unwrap();
            assert_eq!(ret.id, image.id);
            assert!(ret.data.is_empty());
            assert_eq!(ret.hash, image.hash);
            assert_eq!(ret.size, image.size);
            assert_eq!(ret.owner, image.owner);
            assert_eq!(ret.create_time, image.create_time);

            let ret = tx.get_image(image.id, None, false).unwrap();
            assert_eq!(ret.id, image.id);
            assert_eq!(ret.data, image.data);
            assert_eq!(ret.hash, image.hash);
            assert_eq!(ret.size, image.size);
            assert_eq!(ret.owner, image.owner);
            assert_eq!(ret.create_time, image.create_time);

            let ret = tx.get_image(image.id, Some("Alice"), false).unwrap();
            assert_eq!(ret.id, image.id);
            assert_eq!(ret.data, image.data);
            assert_eq!(ret.hash, image.hash);
            assert_eq!(ret.size, image.size);
            assert_eq!(ret.owner, image.owner);
            assert_eq!(ret.create_time, image.create_time);

            // Try to get image not owned by creator, should fail
            let result = tx.get_image(image.id, Some("Bob"), false);
            assert!(result.is_err());
        }

        assert_eq!(tx.count_images(None).unwrap(), images.len());
        assert_eq!(tx.count_images(Some("Alice")).unwrap(), images.len());
        assert_eq!(tx.count_images(Some("Bob")).unwrap(), 0);

        Ok(())
    })
    .unwrap();

    // Put a newer image
    let last_image = db
        .with_transaction(|tx, _cache| {
            advance_mock_time(10);
            let create_image = tx.create_image(mock_image("New image from Alice")).unwrap();
            create_images.push(create_image);

            advance_mock_time(10);
            tx.create_image(ImageRecord {
                id: 0,
                data: "New image from Bob".as_bytes().to_vec(),
                hash: "hash".to_string(),
                owner: "Bob".to_string(),
                size: 5,
                create_time: 0,
            })
        })
        .unwrap();
    create_images.push(last_image.clone());

    // Get the latest image
    db.with_transaction(|tx, _cache| {
        let image = tx.get_latest_image(None, false).unwrap();
        assert_eq!(image, last_image);

        let image = tx.get_latest_image(None, true).unwrap();
        assert!(image.data.is_empty());

        let image = tx.get_latest_image(Some("Alice"), false).unwrap();
        assert_eq!(image.data, "New image from Alice".as_bytes());

        let image = tx.get_latest_image(Some("Bob"), false).unwrap();
        assert_eq!(image.data, "New image from Bob".as_bytes());

        let result = tx.get_latest_image(Some("Charlie"), false);
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();

    let mut images_map = create_images
        .iter()
        .map(|image| (image.id, image.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    for image in images_map.values_mut() {
        image.data = vec![];
    }
    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        let all_images = tx.list_images(query.clone()).unwrap();
        assert_eq!(all_images.len(), images_map.len());

        for image in all_images.iter() {
            let expected = images_map.get(&image.id).unwrap();
            assert_eq!(image, expected);
        }

        // records should be sorted by id (desc)
        let mut sorted = all_images.clone();
        sorted.sort_unstable_by(|a, b| b.id.cmp(&a.id));
        assert_eq!(all_images, sorted);

        query.limit = Some(2);
        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), 2);

        query.offset = Some(all_images.len() as u64 - 2);
        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0], all_images[all_images.len() - 2]);
        assert_eq!(images[1], all_images[all_images.len() - 1]);

        // Search by owner
        let mut query = Query::default();
        query.search = Some("lice".to_string());
        let images = tx.list_images(query.clone()).unwrap();
        assert!(!images.is_empty());
        for image in images.iter() {
            assert!(image.owner.contains("lice"));
        }

        // Search non-existent owner
        query.search = Some("non-existent".to_string());
        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), 0);

        let target_image = images_map.values().next().unwrap();
        let mut query = Query::default();
        query.hash = Some(target_image.hash.clone());
        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], target_image.clone());

        Ok(())
    })
    .unwrap();

    let test_time_start = current_timestamp();

    advance_mock_time(10);
    let start = current_timestamp();
    advance_mock_time(10);
    let mut since_result = db
        .with_transaction(|tx, _cache| tx.create_image(mock_image("To test since")))
        .unwrap();
    since_result.data = vec![];

    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        query.since = Some(start);

        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], since_result);

        Ok(())
    })
    .unwrap();

    advance_mock_time(10);
    let start = current_timestamp();
    advance_mock_time(10);
    let mut range_result = db
        .with_transaction(|tx, _cache| tx.create_image(mock_image("To test since and until")))
        .unwrap();
    range_result.data = vec![];
    advance_mock_time(10);
    let end = current_timestamp();

    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        query.since = Some(start);
        query.until = Some(end);
        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], range_result);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        // Query images before creating since and until mocked.
        let mut query = Query::default();
        query.until = Some(test_time_start);
        let images = tx.list_images(query.clone()).unwrap();
        assert_eq!(images.len(), images_map.len());
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let old_images = tx.get_oldest_image_ids(5).unwrap();
        assert_eq!(old_images, vec![1, 2, 3, 4, 5]);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        tx.delete_image(1).unwrap();
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_image_exists(1, None).unwrap());
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.delete_images_batch(&[2, 3]).unwrap();
        assert_eq!(count, 2);
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_image_exists(2, None).unwrap());
        assert!(!tx.is_image_exists(3, None).unwrap());
        Ok(())
    })
    .unwrap();

    let now = current_timestamp();
    advance_mock_time(10);

    db.with_transaction(|tx, _cache| {
        tx.create_image(mock_image("Survivor")).unwrap();
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.count_images(None).unwrap();
        let deleted = tx.delete_images_before_time(now).unwrap();
        assert_eq!(deleted, count - 1);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.count_images(None).unwrap();
        assert_eq!(count, 1);
        let images = tx.list_images(Query::default()).unwrap();
        assert_eq!(images.len(), 1);
        Ok(())
    })
    .unwrap();
}

fn mock_image(data: &str) -> ImageRecord {
    let data = data.as_bytes().to_vec();
    let hash = Sha256::digest(&data);
    let hash = format!("{:x}", hash);
    let size = data.len() as u64;
    ImageRecord {
        id: 0,
        data,
        hash,
        size,
        owner: "Alice".to_string(),
        create_time: 0,
    }
}
