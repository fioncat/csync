use csync_misc::types::request::Query;
use sha2::{Digest, Sha256};

use crate::db::{Database, FileRecord};
use crate::now::{advance_mock_time, current_timestamp};

pub fn run_file_tests(db: &Database) {
    let files = [
        mock_file("test", "content", 0o644),
        mock_file("file1", "content1", 0o644),
        mock_file("file2", "content2", 0o644),
        mock_file("file3", "content3", 0o644),
        mock_file("file4", "content4", 0o644),
        mock_file("file5", "content5", 0o644),
        mock_file("file6", "content6", 0o644),
        // Allow duplicate file names
        mock_file("test", "content_test222", 0o644),
    ];

    let mut create_files = vec![];
    db.with_transaction(|tx, _cache| {
        for file in files.iter() {
            let ret = tx.create_file(file.clone()).unwrap();
            create_files.push(ret);
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(create_files.len(), files.len());
    db.with_transaction(|tx, _cache| {
        for file in create_files.iter() {
            let ret = tx.get_file(file.id, None, true).unwrap();
            assert_eq!(ret.id, file.id);
            assert!(ret.data.is_empty());
            assert_eq!(ret.hash, file.hash);
            assert_eq!(ret.name, file.name);
            assert_eq!(ret.mode, file.mode);
            assert_eq!(ret.size, file.size);
            assert_eq!(ret.owner, file.owner);
            assert_eq!(ret.create_time, file.create_time);

            let ret = tx.get_file(file.id, None, false).unwrap();
            assert_eq!(ret, file.clone());

            let ret = tx.get_file(file.id, Some("Alice"), false).unwrap();
            assert_eq!(ret, file.clone());

            // Try to get file not owned by creator, should fail
            let result = tx.get_file(file.id, Some("Bob"), false);
            assert!(result.is_err());
        }

        assert_eq!(tx.count_files(None).unwrap(), files.len());
        assert_eq!(tx.count_files(Some("Alice")).unwrap(), files.len());
        assert_eq!(tx.count_files(Some("Bob")).unwrap(), 0);

        Ok(())
    })
    .unwrap();

    // Put a newer file
    let last_file = db
        .with_transaction(|tx, _cache| {
            advance_mock_time(10);
            let create_file = tx
                .create_file(mock_file("new_file_alice", "New file from Alice", 0o777))
                .unwrap();
            create_files.push(create_file);

            advance_mock_time(10);
            tx.create_file(FileRecord {
                id: 0,
                name: "new_file_bob".to_string(),
                data: "New file from Bob".as_bytes().to_vec(),
                mode: 0o666,
                hash: "hash".to_string(),
                owner: "Bob".to_string(),
                size: 5,
                create_time: 0,
            })
        })
        .unwrap();
    create_files.push(last_file.clone());

    // Get the latest file
    db.with_transaction(|tx, _cache| {
        let file = tx.get_latest_file(None, false).unwrap();
        assert_eq!(file, last_file);

        let file = tx.get_latest_file(None, true).unwrap();
        assert!(file.data.is_empty());

        let file = tx.get_latest_file(Some("Alice"), false).unwrap();
        assert_eq!(file.name, "new_file_alice");
        assert_eq!(file.data, "New file from Alice".as_bytes());
        assert_eq!(file.mode, 0o777);

        let file = tx.get_latest_file(Some("Bob"), false).unwrap();
        assert_eq!(file.name, "new_file_bob");
        assert_eq!(file.data, "New file from Bob".as_bytes());
        assert_eq!(file.mode, 0o666);

        let result = tx.get_latest_file(Some("Charlie"), false);
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();

    let mut files_map = create_files
        .iter()
        .map(|file| (file.id, file.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    for file in files_map.values_mut() {
        file.data = vec![];
    }
    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        let all_files = tx.list_files(query.clone()).unwrap();
        assert_eq!(all_files.len(), files_map.len());

        for file in all_files.iter() {
            let expected = files_map.get(&file.id).unwrap();
            assert_eq!(file, expected);
        }

        // records should be sorted by id (desc)
        let mut sorted = all_files.clone();
        sorted.sort_unstable_by(|a, b| b.id.cmp(&a.id));
        assert_eq!(all_files, sorted);

        query.limit = Some(2);
        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), 2);

        query.offset = Some(all_files.len() as u64 - 2);
        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], all_files[all_files.len() - 2]);
        assert_eq!(files[1], all_files[all_files.len() - 1]);

        // Search by owner
        let mut query = Query::default();
        query.search = Some("lice".to_string());
        let files = tx.list_files(query.clone()).unwrap();
        assert!(!files.is_empty());
        for file in files.iter() {
            assert!(file.owner.contains("lice"));
        }

        // Search non-existent owner
        query.search = Some("non-existent".to_string());
        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), 0);

        let target_file = files_map.values().next().unwrap();
        let mut query = Query::default();
        query.hash = Some(target_file.hash.clone());
        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], target_file.clone());

        Ok(())
    })
    .unwrap();

    let test_time_start = current_timestamp();

    advance_mock_time(10);
    let start = current_timestamp();
    advance_mock_time(10);
    let mut since_result = db
        .with_transaction(|tx, _cache| {
            tx.create_file(mock_file("test_since", "To test since", 0o644))
        })
        .unwrap();
    since_result.data = vec![];

    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        query.since = Some(start);

        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], since_result);

        Ok(())
    })
    .unwrap();

    advance_mock_time(10);
    let start = current_timestamp();
    advance_mock_time(10);
    let mut range_result = db
        .with_transaction(|tx, _cache| {
            tx.create_file(mock_file(
                "test_since_until",
                "To test since and until",
                0o644,
            ))
        })
        .unwrap();
    range_result.data = vec![];
    advance_mock_time(10);
    let end = current_timestamp();

    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        query.since = Some(start);
        query.until = Some(end);
        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], range_result);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        // Query files before creating since and until mocked.
        let mut query = Query::default();
        query.until = Some(test_time_start);
        let files = tx.list_files(query.clone()).unwrap();
        assert_eq!(files.len(), files_map.len());
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let old_files = tx.get_oldest_file_ids(5).unwrap();
        assert_eq!(old_files, vec![1, 2, 3, 4, 5]);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        tx.delete_file(1).unwrap();
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_file_exists(1, None).unwrap());
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.delete_files_batch(&[2, 3]).unwrap();
        assert_eq!(count, 2);
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_file_exists(2, None).unwrap());
        assert!(!tx.is_file_exists(3, None).unwrap());
        Ok(())
    })
    .unwrap();

    let now = current_timestamp();
    advance_mock_time(10);

    db.with_transaction(|tx, _cache| {
        tx.create_file(mock_file("survivor", "Survivor", 0o777))
            .unwrap();
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.count_files(None).unwrap();
        let deleted = tx.delete_files_before_time(now).unwrap();
        assert_eq!(deleted, count - 1);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.count_files(None).unwrap();
        assert_eq!(count, 1);
        let files = tx.list_files(Query::default()).unwrap();
        assert_eq!(files.len(), 1);
        Ok(())
    })
    .unwrap();
}

pub fn mock_file(name: &str, content: &str, mode: u32) -> FileRecord {
    let data = content.as_bytes().to_vec();
    let hash = Sha256::digest(&data);
    let hash = format!("{:x}", hash);
    let size = data.len() as u64;
    FileRecord {
        id: 0,
        name: name.to_string(),
        data,
        hash,
        mode,
        size,
        owner: "Alice".to_string(),
        create_time: 0,
    }
}
