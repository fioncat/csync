use csync_misc::types::user::RoleRule;

use crate::db::{FileRecord, ImageRecord, RoleRecord, TextRecord};

use super::Cache;

pub fn run_all_cache_tests(cache: &dyn Cache) {
    test_user(cache);
    test_text(cache);
    test_image(cache);
    test_file(cache);
}

fn test_user(cache: &dyn Cache) {
    let roles = cache.list_user_roles("user").unwrap();
    assert_eq!(roles, None);

    let roles = vec![
        RoleRecord {
            name: "test".to_string(),
            rules: vec![
                RoleRule {
                    resources: ["test".to_string()].into_iter().collect(),
                    verbs: ["put".to_string(), "get".to_string()].into_iter().collect(),
                },
                RoleRule {
                    resources: ["image".to_string()].into_iter().collect(),
                    verbs: ["*".to_string()].into_iter().collect(),
                },
            ],
            create_time: 0,
            update_time: 0,
        },
        RoleRecord {
            name: "wheel".to_string(),
            rules: vec![RoleRule {
                resources: ["*".to_string()].into_iter().collect(),
                verbs: ["*".to_string()].into_iter().collect(),
            }],
            create_time: 0,
            update_time: 0,
        },
    ];
    cache.save_user_roles("user", roles.clone()).unwrap();
    cache.save_user_roles("test", vec![]).unwrap();

    let result_roles = cache.list_user_roles("user").unwrap().unwrap();
    assert_eq!(result_roles, roles);

    assert_eq!(cache.list_user_roles("test").unwrap(), Some(vec![]));

    cache.delete_user_roles("user").unwrap();
    assert_eq!(cache.list_user_roles("user").unwrap(), None);
}

fn test_text(cache: &dyn Cache) {
    assert_eq!(cache.get_latest_text(None).unwrap(), None);
    assert_eq!(cache.get_latest_text(Some("Alice")).unwrap(), None);

    let alice_text = TextRecord {
        id: 1,
        owner: "Alice".to_string(),
        content: "Hello".to_string(),
        hash: "hash".to_string(),
        size: 5,
        create_time: 0,
    };
    let bob_text = TextRecord {
        id: 2,
        owner: "Bob".to_string(),
        content: "World".to_string(),
        hash: "hash".to_string(),
        size: 5,
        create_time: 0,
    };
    let charlie_text = TextRecord {
        id: 3,
        owner: "Charlie".to_string(),
        content: "Hello World".to_string(),
        hash: "hash".to_string(),
        size: 11,
        create_time: 0,
    };

    cache.save_latest_text("Alice", alice_text.clone()).unwrap();
    cache.save_latest_text("Bob", bob_text.clone()).unwrap();
    cache
        .save_latest_text("Charlie", charlie_text.clone())
        .unwrap();

    assert_eq!(
        cache.get_latest_text(None).unwrap(),
        Some(charlie_text.clone())
    );
    assert_eq!(
        cache.get_latest_text(Some("Alice")).unwrap(),
        Some(alice_text)
    );
    assert_eq!(cache.get_latest_text(Some("Bob")).unwrap(), Some(bob_text));
    assert_eq!(
        cache.get_latest_text(Some("Charlie")).unwrap(),
        Some(charlie_text.clone())
    );
    assert_eq!(cache.get_latest_text(Some("David")).unwrap(), None);

    cache.delete_latest_text("Charlie").unwrap();
    assert_eq!(cache.get_latest_text(Some("Charlie")).unwrap(), None);
    assert_eq!(cache.get_latest_text(None).unwrap(), None);
    cache
        .save_latest_text("Charlie", charlie_text.clone())
        .unwrap();

    cache.clear_text().unwrap();
    assert_eq!(cache.get_latest_text(None).unwrap(), None);
    assert_eq!(cache.get_latest_text(Some("Alice")).unwrap(), None);
    assert_eq!(cache.get_latest_text(Some("Bob")).unwrap(), None);
    assert_eq!(cache.get_latest_text(Some("Charlie")).unwrap(), None);
}

fn test_image(cache: &dyn Cache) {
    assert_eq!(cache.get_latest_image(None).unwrap(), None);
    assert_eq!(cache.get_latest_image(Some("Alice")).unwrap(), None);

    let alice_image = ImageRecord {
        id: 1,
        owner: "Alice".to_string(),
        data: "Hello".as_bytes().to_vec(),
        hash: "hash".to_string(),
        size: 5,
        create_time: 0,
    };
    let bob_image = ImageRecord {
        id: 2,
        owner: "Bob".to_string(),
        data: "World".as_bytes().to_vec(),
        hash: "hash".to_string(),
        size: 5,
        create_time: 0,
    };
    let charlie_image = ImageRecord {
        id: 3,
        owner: "Charlie".to_string(),
        data: "Hello World".as_bytes().to_vec(),
        hash: "hash".to_string(),
        size: 11,
        create_time: 0,
    };

    cache
        .save_latest_image("Alice", alice_image.clone())
        .unwrap();
    cache.save_latest_image("Bob", bob_image.clone()).unwrap();
    cache
        .save_latest_image("Charlie", charlie_image.clone())
        .unwrap();

    assert_eq!(
        cache.get_latest_image(None).unwrap(),
        Some(charlie_image.clone())
    );
    assert_eq!(
        cache.get_latest_image(Some("Alice")).unwrap(),
        Some(alice_image)
    );
    assert_eq!(
        cache.get_latest_image(Some("Bob")).unwrap(),
        Some(bob_image)
    );
    assert_eq!(
        cache.get_latest_image(Some("Charlie")).unwrap(),
        Some(charlie_image.clone())
    );
    assert_eq!(cache.get_latest_image(Some("David")).unwrap(), None);

    cache.delete_latest_image("Charlie").unwrap();
    assert_eq!(cache.get_latest_image(Some("Charlie")).unwrap(), None);
    assert_eq!(cache.get_latest_image(None).unwrap(), None);
    cache
        .save_latest_image("Charlie", charlie_image.clone())
        .unwrap();

    cache.clear_image().unwrap();
    assert_eq!(cache.get_latest_image(None).unwrap(), None);
    assert_eq!(cache.get_latest_image(Some("Alice")).unwrap(), None);
    assert_eq!(cache.get_latest_image(Some("Bob")).unwrap(), None);
    assert_eq!(cache.get_latest_image(Some("Charlie")).unwrap(), None);
}

fn test_file(cache: &dyn Cache) {
    assert_eq!(cache.get_latest_file(None).unwrap(), None);
    assert_eq!(cache.get_latest_file(Some("Alice")).unwrap(), None);

    let alice_file = FileRecord {
        id: 1,
        name: "alice_file".to_string(),
        owner: "Alice".to_string(),
        data: "Hello".as_bytes().to_vec(),
        hash: "hash".to_string(),
        mode: 0o644,
        size: 5,
        create_time: 0,
    };
    let bob_file = FileRecord {
        id: 2,
        name: "bob_file".to_string(),
        owner: "Bob".to_string(),
        data: "World".as_bytes().to_vec(),
        hash: "hash".to_string(),
        mode: 0o666,
        size: 5,
        create_time: 0,
    };
    let charlie_file = FileRecord {
        id: 3,
        name: "charlie_file".to_string(),
        owner: "Charlie".to_string(),
        data: "Hello World".as_bytes().to_vec(),
        hash: "hash".to_string(),
        mode: 0o777,
        size: 11,
        create_time: 0,
    };

    cache.save_latest_file("Alice", alice_file.clone()).unwrap();
    cache.save_latest_file("Bob", bob_file.clone()).unwrap();
    cache
        .save_latest_file("Charlie", charlie_file.clone())
        .unwrap();

    assert_eq!(
        cache.get_latest_file(None).unwrap(),
        Some(charlie_file.clone())
    );
    assert_eq!(
        cache.get_latest_file(Some("Alice")).unwrap(),
        Some(alice_file)
    );
    assert_eq!(cache.get_latest_file(Some("Bob")).unwrap(), Some(bob_file));
    assert_eq!(
        cache.get_latest_file(Some("Charlie")).unwrap(),
        Some(charlie_file.clone())
    );
    assert_eq!(cache.get_latest_file(Some("David")).unwrap(), None);

    cache.delete_latest_file("Charlie").unwrap();
    assert_eq!(cache.get_latest_file(Some("Charlie")).unwrap(), None);
    assert_eq!(cache.get_latest_file(None).unwrap(), None);
    cache
        .save_latest_file("Charlie", charlie_file.clone())
        .unwrap();

    cache.clear_file().unwrap();
    assert_eq!(cache.get_latest_file(None).unwrap(), None);
    assert_eq!(cache.get_latest_file(Some("Alice")).unwrap(), None);
    assert_eq!(cache.get_latest_file(Some("Bob")).unwrap(), None);
    assert_eq!(cache.get_latest_file(Some("Charlie")).unwrap(), None);
}
