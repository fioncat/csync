use std::cell::RefCell;
use std::sync::Mutex;

use anyhow::Result;

use super::RevisionProvider;

pub struct MemoryRevision {
    rev: Mutex<RefCell<String>>,
}

impl MemoryRevision {
    pub fn new() -> Self {
        Self {
            rev: Mutex::new(RefCell::new(String::new())),
        }
    }
}

impl RevisionProvider for MemoryRevision {
    fn save(&self, rev: String) -> Result<()> {
        self.rev.lock().unwrap().replace(rev);
        Ok(())
    }

    fn load(&self) -> Result<String> {
        let rev = self.rev.lock().unwrap().borrow().clone();
        Ok(rev)
    }
}
