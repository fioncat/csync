pub mod config;
pub mod factory;

mod memory;

use anyhow::Result;
use memory::MemoryRevision;

pub enum Revision {
    Memory(MemoryRevision),
}

pub trait Revisier {
    fn update(&self) -> Result<()>;
    fn load(&self) -> Result<u64>;
}

impl Revisier for Revision {
    fn update(&self) -> Result<()> {
        match self {
            Self::Memory(r) => r.update(),
        }
    }

    fn load(&self) -> Result<u64> {
        match self {
            Self::Memory(r) => r.load(),
        }
    }
}
