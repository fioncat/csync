mod memory;

#[cfg(test)]
mod tests;

pub mod config;
pub mod factory;

use anyhow::Result;
use memory::MemoryCache;

use super::{FileRecord, ImageRecord, RoleRecord, TextRecord};

pub trait Cache {
    fn list_user_roles(&self, user: &str) -> Result<Option<Vec<RoleRecord>>>;
    fn save_user_roles(&self, user: &str, roles: Vec<RoleRecord>) -> Result<()>;
    fn delete_user_roles(&self, user: &str) -> Result<()>;

    fn get_latest_text(&self, owner: Option<&str>) -> Result<Option<TextRecord>>;
    fn save_latest_text(&self, owner: &str, text: TextRecord) -> Result<()>;
    fn delete_latest_text(&self, owner: &str) -> Result<()>;
    fn clear_text(&self) -> Result<()>;

    fn get_latest_image(&self, owner: Option<&str>) -> Result<Option<ImageRecord>>;
    fn save_latest_image(&self, owner: &str, image: ImageRecord) -> Result<()>;
    fn delete_latest_image(&self, owner: &str) -> Result<()>;
    fn clear_image(&self) -> Result<()>;

    fn get_latest_file(&self, owner: Option<&str>) -> Result<Option<FileRecord>>;
    fn save_latest_file(&self, owner: &str, image: FileRecord) -> Result<()>;
    fn delete_latest_file(&self, owner: &str) -> Result<()>;
    fn clear_file(&self) -> Result<()>;
}

pub enum UnionCache {
    Memory(MemoryCache),
}

impl Cache for UnionCache {
    fn list_user_roles(&self, user: &str) -> Result<Option<Vec<RoleRecord>>> {
        match self {
            Self::Memory(cache) => cache.list_user_roles(user),
        }
    }

    fn save_user_roles(&self, user: &str, roles: Vec<RoleRecord>) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.save_user_roles(user, roles),
        }
    }

    fn delete_user_roles(&self, user: &str) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.delete_user_roles(user),
        }
    }

    fn get_latest_text(&self, owner: Option<&str>) -> Result<Option<TextRecord>> {
        match self {
            Self::Memory(cache) => cache.get_latest_text(owner),
        }
    }

    fn save_latest_text(&self, owner: &str, text: TextRecord) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.save_latest_text(owner, text),
        }
    }

    fn delete_latest_text(&self, owner: &str) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.delete_latest_text(owner),
        }
    }

    fn clear_text(&self) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.clear_text(),
        }
    }

    fn get_latest_image(&self, owner: Option<&str>) -> Result<Option<ImageRecord>> {
        match self {
            Self::Memory(cache) => cache.get_latest_image(owner),
        }
    }

    fn save_latest_image(&self, owner: &str, image: ImageRecord) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.save_latest_image(owner, image),
        }
    }

    fn delete_latest_image(&self, owner: &str) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.delete_latest_image(owner),
        }
    }

    fn clear_image(&self) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.clear_image(),
        }
    }

    fn get_latest_file(&self, owner: Option<&str>) -> Result<Option<FileRecord>> {
        match self {
            Self::Memory(cache) => cache.get_latest_file(owner),
        }
    }

    fn save_latest_file(&self, owner: &str, image: FileRecord) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.save_latest_file(owner, image),
        }
    }

    fn delete_latest_file(&self, owner: &str) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.delete_latest_file(owner),
        }
    }

    fn clear_file(&self) -> Result<()> {
        match self {
            Self::Memory(cache) => cache.clear_file(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisableCache;

impl Cache for DisableCache {
    fn list_user_roles(&self, _user: &str) -> Result<Option<Vec<RoleRecord>>> {
        Ok(None)
    }

    fn save_user_roles(&self, _user: &str, _roles: Vec<RoleRecord>) -> Result<()> {
        Ok(())
    }

    fn delete_user_roles(&self, _user: &str) -> Result<()> {
        Ok(())
    }

    fn get_latest_text(&self, _owner: Option<&str>) -> Result<Option<TextRecord>> {
        Ok(None)
    }

    fn save_latest_text(&self, _owner: &str, _text: TextRecord) -> Result<()> {
        Ok(())
    }

    fn delete_latest_text(&self, _owner: &str) -> Result<()> {
        Ok(())
    }

    fn clear_text(&self) -> Result<()> {
        Ok(())
    }

    fn get_latest_image(&self, _owner: Option<&str>) -> Result<Option<ImageRecord>> {
        Ok(None)
    }

    fn save_latest_image(&self, _owner: &str, _image: ImageRecord) -> Result<()> {
        Ok(())
    }

    fn delete_latest_image(&self, _owner: &str) -> Result<()> {
        Ok(())
    }

    fn clear_image(&self) -> Result<()> {
        Ok(())
    }

    fn get_latest_file(&self, _owner: Option<&str>) -> Result<Option<FileRecord>> {
        Ok(None)
    }

    fn save_latest_file(&self, _owner: &str, _image: FileRecord) -> Result<()> {
        Ok(())
    }

    fn delete_latest_file(&self, _owner: &str) -> Result<()> {
        Ok(())
    }

    fn clear_file(&self) -> Result<()> {
        Ok(())
    }
}
