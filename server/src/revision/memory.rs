use std::cell::RefCell;
use std::sync::Mutex;

use anyhow::Result;

use super::Revisier;

pub struct MemoryRevision {
    rev: Mutex<RefCell<u64>>,
}

impl MemoryRevision {
    pub fn new() -> Self {
        Self {
            rev: Mutex::new(RefCell::new(0)),
        }
    }
}

impl Revisier for MemoryRevision {
    fn update(&self) -> Result<()> {
        let rev = self.rev.lock().unwrap();
        let new_rev = *rev.borrow() + 1;
        rev.replace(new_rev);
        Ok(())
    }

    fn load(&self) -> Result<u64> {
        Ok(*self.rev.lock().unwrap().borrow())
    }
}
