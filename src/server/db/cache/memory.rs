use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;

use crate::server::db::{FileRecord, ImageRecord, RoleRecord, TextRecord};

use super::Cache;

/// A memory-based cache implementation that stores data in simple maps.
/// The cached data is stored in memory and does not expire.
pub struct MemoryCache {
    user_roles: RefCell<HashMap<String, Vec<RoleRecord>>>,

    latest_text: RefCell<Option<Arc<TextRecord>>>,
    latest_text_owners: RefCell<HashMap<String, Arc<TextRecord>>>,

    latest_image: RefCell<Option<Arc<ImageRecord>>>,
    latest_image_owners: RefCell<HashMap<String, Arc<ImageRecord>>>,

    latest_file: RefCell<Option<Arc<FileRecord>>>,
    latest_file_owners: RefCell<HashMap<String, Arc<FileRecord>>>,
}

impl MemoryCache {
    /// Creates a new instance of memory cache.
    pub fn new() -> Self {
        Self {
            user_roles: RefCell::new(HashMap::new()),
            latest_text: RefCell::new(None),
            latest_text_owners: RefCell::new(HashMap::new()),
            latest_image: RefCell::new(None),
            latest_image_owners: RefCell::new(HashMap::new()),
            latest_file: RefCell::new(None),
            latest_file_owners: RefCell::new(HashMap::new()),
        }
    }
}

impl Cache for MemoryCache {
    fn list_user_roles(&self, user: &str) -> Result<Option<Vec<RoleRecord>>> {
        Ok(self.user_roles.borrow().get(user).cloned())
    }

    fn save_user_roles(&self, user: &str, roles: Vec<RoleRecord>) -> Result<()> {
        self.user_roles.borrow_mut().insert(user.to_string(), roles);
        Ok(())
    }

    fn delete_user_roles(&self, user: &str) -> Result<()> {
        self.user_roles.borrow_mut().remove(user);
        Ok(())
    }

    fn get_latest_text(&self, owner: Option<&str>) -> Result<Option<TextRecord>> {
        match owner {
            Some(owner) => Ok(self
                .latest_text_owners
                .borrow()
                .get(owner)
                .map(|r| r.as_ref().clone())),
            None => Ok(self
                .latest_text
                .borrow()
                .as_ref()
                .map(|r| r.as_ref().clone())),
        }
    }

    fn save_latest_text(&self, owner: &str, text: TextRecord) -> Result<()> {
        let text = Arc::new(text);
        self.latest_text_owners
            .borrow_mut()
            .insert(owner.to_string(), text.clone());
        self.latest_text.replace(Some(text));
        Ok(())
    }

    fn delete_latest_text(&self, owner: &str) -> Result<()> {
        self.latest_text_owners.borrow_mut().remove(owner);
        if self
            .latest_text
            .borrow()
            .as_ref()
            .is_some_and(|r| r.owner == owner)
        {
            self.latest_text.replace(None);
        }
        Ok(())
    }

    fn clear_text(&self) -> Result<()> {
        self.latest_text_owners.borrow_mut().clear();
        self.latest_text.replace(None);
        Ok(())
    }

    fn get_latest_image(&self, owner: Option<&str>) -> Result<Option<ImageRecord>> {
        match owner {
            Some(owner) => Ok(self
                .latest_image_owners
                .borrow()
                .get(owner)
                .map(|r| r.as_ref().clone())),
            None => Ok(self
                .latest_image
                .borrow()
                .as_ref()
                .map(|r| r.as_ref().clone())),
        }
    }

    fn save_latest_image(&self, owner: &str, image: ImageRecord) -> Result<()> {
        let image = Arc::new(image);
        self.latest_image_owners
            .borrow_mut()
            .insert(owner.to_string(), image.clone());
        self.latest_image.replace(Some(image));
        Ok(())
    }

    fn delete_latest_image(&self, owner: &str) -> Result<()> {
        self.latest_image_owners.borrow_mut().remove(owner);
        if self
            .latest_image
            .borrow()
            .as_ref()
            .is_some_and(|r| r.owner == owner)
        {
            self.latest_image.replace(None);
        }
        Ok(())
    }

    fn clear_image(&self) -> Result<()> {
        self.latest_image_owners.borrow_mut().clear();
        self.latest_image.replace(None);
        Ok(())
    }

    fn get_latest_file(&self, owner: Option<&str>) -> Result<Option<FileRecord>> {
        match owner {
            Some(owner) => Ok(self
                .latest_file_owners
                .borrow()
                .get(owner)
                .map(|r| r.as_ref().clone())),
            None => Ok(self
                .latest_file
                .borrow()
                .as_ref()
                .map(|r| r.as_ref().clone())),
        }
    }

    fn save_latest_file(&self, owner: &str, file: FileRecord) -> Result<()> {
        let file = Arc::new(file);
        self.latest_file_owners
            .borrow_mut()
            .insert(owner.to_string(), file.clone());
        self.latest_file.replace(Some(file));
        Ok(())
    }

    fn delete_latest_file(&self, owner: &str) -> Result<()> {
        self.latest_file_owners.borrow_mut().remove(owner);
        if self
            .latest_file
            .borrow()
            .as_ref()
            .is_some_and(|r| r.owner == owner)
        {
            self.latest_file.replace(None);
        }
        Ok(())
    }

    fn clear_file(&self) -> Result<()> {
        self.latest_file_owners.borrow_mut().clear();
        self.latest_file.replace(None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::server::db::cache::tests::run_all_cache_tests;

    use super::*;

    #[test]
    fn test_memory() {
        let cache = MemoryCache::new();
        run_all_cache_tests(&cache);
    }
}
