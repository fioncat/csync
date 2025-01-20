use sha2::{Digest, Sha256};

use crate::server::db::{Database, TextRecord};
use crate::time::{advance_mock_time, current_timestamp};
use crate::types::request::Query;

pub fn run_text_tests(db: &Database) {
    let texts = [
        mock_text("Hello, world!"),
        mock_text("Hello, Rust!"),
        mock_text("This is a simple text\nWith next line"),
        mock_text(""),
        mock_text("\n\n\t"),
        mock_text("Test text!!"),
        mock_text("Happy coding!"),
    ];

    let mut create_texts = vec![];
    db.with_transaction(|tx, _cache| {
        for text in texts.iter() {
            let ret = tx.create_text(text.clone()).unwrap();
            create_texts.push(ret);
        }
        Ok(())
    })
    .unwrap();

    assert_eq!(create_texts.len(), texts.len());
    db.with_transaction(|tx, _cache| {
        for text in create_texts.iter() {
            let ret = tx.get_text(text.id, None, true).unwrap();
            assert_eq!(ret.id, text.id);
            assert_eq!(ret.content, "");
            assert_eq!(ret.hash, text.hash);
            assert_eq!(ret.size, text.size);
            assert_eq!(ret.owner, text.owner);
            assert_eq!(ret.create_time, text.create_time);

            let ret = tx.get_text(text.id, None, false).unwrap();
            assert_eq!(ret.id, text.id);
            assert_eq!(ret.content, text.content);
            assert_eq!(ret.hash, text.hash);
            assert_eq!(ret.size, text.size);
            assert_eq!(ret.owner, text.owner);
            assert_eq!(ret.create_time, text.create_time);

            let ret = tx.get_text(text.id, Some("Alice"), false).unwrap();
            assert_eq!(ret.id, text.id);
            assert_eq!(ret.content, text.content);
            assert_eq!(ret.hash, text.hash);
            assert_eq!(ret.size, text.size);
            assert_eq!(ret.owner, text.owner);
            assert_eq!(ret.create_time, text.create_time);

            // Try to get text not owned by creator, should fail
            let result = tx.get_text(text.id, Some("Bob"), false);
            assert!(result.is_err());
        }

        assert_eq!(tx.count_texts(None).unwrap(), texts.len());
        assert_eq!(tx.count_texts(Some("Alice")).unwrap(), texts.len());
        assert_eq!(tx.count_texts(Some("Bob")).unwrap(), 0);

        Ok(())
    })
    .unwrap();

    // Put a newer text
    let last_text = db
        .with_transaction(|tx, _cache| {
            advance_mock_time(10);
            let create_text = tx.create_text(mock_text("New text from Alice")).unwrap();
            create_texts.push(create_text);

            advance_mock_time(10);
            tx.create_text(TextRecord {
                id: 0,
                content: "New text from Bob".to_string(),
                hash: "hash".to_string(),
                owner: "Bob".to_string(),
                size: 5,
                create_time: 0,
            })
        })
        .unwrap();
    create_texts.push(last_text.clone());

    // Get the latest text
    db.with_transaction(|tx, _cache| {
        let text = tx.get_latest_text(None, false).unwrap();
        assert_eq!(text, last_text);

        let text = tx.get_latest_text(None, true).unwrap();
        assert_eq!(text.content, "");

        let text = tx.get_latest_text(Some("Alice"), false).unwrap();
        assert_eq!(text.content, "New text from Alice");

        let text = tx.get_latest_text(Some("Bob"), false).unwrap();
        assert_eq!(text.content, "New text from Bob");

        let result = tx.get_latest_text(Some("Charlie"), false);
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();

    let texts_map = create_texts
        .iter()
        .map(|text| (text.id, text.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        let all_texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(all_texts.len(), texts_map.len());

        for text in all_texts.iter() {
            let expected = texts_map.get(&text.id).unwrap();
            assert_eq!(text, expected);
        }

        // records should be sorted by id (desc)
        let mut sorted = all_texts.clone();
        sorted.sort_unstable_by(|a, b| b.id.cmp(&a.id));
        assert_eq!(all_texts, sorted);

        query.limit = Some(2);
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), 2);

        query.offset = Some(all_texts.len() as u64 - 2);
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), 2);
        assert_eq!(texts[0], all_texts[all_texts.len() - 2]);
        assert_eq!(texts[1], all_texts[all_texts.len() - 1]);

        // Search by owner
        let mut query = Query::default();
        query.search = Some("lice".to_string());
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert!(!texts.is_empty());
        for text in texts.iter() {
            assert!(text.owner.contains("lice"));
        }

        // Search non-existent owner
        query.search = Some("non-existent".to_string());
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), 0);

        let target_text = texts_map.values().next().unwrap();
        let mut query = Query::default();
        query.hash = Some(target_text.hash.clone());
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0], target_text.clone());

        // Simple search
        let texts = tx.list_texts(Query::default(), true).unwrap();
        assert_eq!(texts.len(), texts_map.len());
        for text in texts.iter() {
            assert_eq!(text.content, "");
        }

        Ok(())
    })
    .unwrap();

    let test_time_start = current_timestamp();

    advance_mock_time(10);
    let start = current_timestamp();
    advance_mock_time(10);
    let since_result = db
        .with_transaction(|tx, _cache| tx.create_text(mock_text("To test since")))
        .unwrap();

    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        query.since = Some(start);

        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0], since_result);

        Ok(())
    })
    .unwrap();

    advance_mock_time(10);
    let start = current_timestamp();
    advance_mock_time(10);
    let range_result = db
        .with_transaction(|tx, _cache| tx.create_text(mock_text("To test since and until")))
        .unwrap();
    advance_mock_time(10);
    let end = current_timestamp();

    db.with_transaction(|tx, _cache| {
        let mut query = Query::default();
        query.since = Some(start);
        query.until = Some(end);
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0], range_result);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        // Query texts before creating since and until mocked.
        let mut query = Query::default();
        query.until = Some(test_time_start);
        let texts = tx.list_texts(query.clone(), false).unwrap();
        assert_eq!(texts.len(), texts_map.len());
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let old_texts = tx.get_oldest_text_ids(5).unwrap();
        assert_eq!(old_texts, vec![1, 2, 3, 4, 5]);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        tx.delete_text(1).unwrap();
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_text_exists(1, None).unwrap());
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.delete_texts_batch(&[2, 3]).unwrap();
        assert_eq!(count, 2);
        Ok(())
    })
    .unwrap();
    db.with_transaction(|tx, _cache| {
        assert!(!tx.is_text_exists(2, None).unwrap());
        assert!(!tx.is_text_exists(3, None).unwrap());
        Ok(())
    })
    .unwrap();

    let now = current_timestamp();
    advance_mock_time(10);

    db.with_transaction(|tx, _cache| {
        tx.create_text(mock_text("Survivor")).unwrap();
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.count_texts(None).unwrap();
        let deleted = tx.delete_texts_before_time(now).unwrap();
        assert_eq!(deleted, count - 1);
        Ok(())
    })
    .unwrap();

    db.with_transaction(|tx, _cache| {
        let count = tx.count_texts(None).unwrap();
        assert_eq!(count, 1);
        let texts = tx.list_texts(Query::default(), false).unwrap();
        assert_eq!(texts.len(), 1);

        let text = &texts[0];
        assert_eq!(text.content, "Survivor");

        Ok(())
    })
    .unwrap();
}

fn mock_text(text: &str) -> TextRecord {
    let hash = Sha256::digest(text.as_bytes());
    let hash = format!("{:x}", hash);
    TextRecord {
        id: 0,
        content: text.to_string(),
        hash,
        size: text.len() as u64,
        owner: "Alice".to_string(),
        create_time: 0,
    }
}
