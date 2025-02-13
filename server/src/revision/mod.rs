pub mod config;
pub mod factory;

mod memory;

use anyhow::Result;
use memory::MemoryRevision;
use uuid::Uuid;

pub enum Revision {
    Memory(MemoryRevision),
}

trait RevisionProvider {
    fn save(&self, rev: String) -> Result<()>;
    fn load(&self) -> Result<String>;
}

impl Revision {
    pub fn update(&self) -> Result<()> {
        let uuid = Uuid::new_v4().to_string().replace("-", "");
        match self {
            Self::Memory(r) => r.save(uuid),
        }
    }

    pub fn load(&self) -> Result<String> {
        match self {
            Self::Memory(r) => r.load(),
        }
    }
}
