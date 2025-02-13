use anyhow::Result;

use super::config::{RevisionConfig, RevisionType};
use super::memory::MemoryRevision;
use super::Revision;

#[derive(Debug, Default)]
pub struct RevisionFactory;

impl RevisionFactory {
    pub fn build_revision(&self, cfg: &RevisionConfig) -> Result<Revision> {
        let revision = match cfg.name {
            RevisionType::Memory => Revision::Memory(MemoryRevision::new()),
        };
        Ok(revision)
    }
}
