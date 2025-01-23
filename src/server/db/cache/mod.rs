#[cfg(test)]
mod tests;

pub mod config;
pub mod factory;
pub mod memory;

use anyhow::Result;
use memory::MemoryCache;

use super::{FileRecord, ImageRecord, RoleRecord, TextRecord};

/// Cache is used to store frequently accessed data to improve server response time.
/// External users can optionally enable or disable the cache.
pub trait Cache {
    /// Get role list by user. This is a high-frequency operation as authentication
    /// is required for every API call, making it a good candidate for caching.
    fn list_user_roles(&self, user: &str) -> Result<Option<Vec<RoleRecord>>>;

    /// Cache the role list for a user.
    fn save_user_roles(&self, user: &str, roles: Vec<RoleRecord>) -> Result<()>;

    /// Remove the cached role list for a user.
    fn delete_user_roles(&self, user: &str) -> Result<()>;

    /// Get the latest text record for a user. If owner is None, returns the latest text record
    /// across all users (typically used by admin users).
    fn get_latest_text(&self, owner: Option<&str>) -> Result<Option<TextRecord>>;

    /// Cache the latest text record for a user.
    fn save_latest_text(&self, owner: &str, text: TextRecord) -> Result<()>;

    /// Remove the cached latest text record for a user.
    fn delete_latest_text(&self, owner: &str) -> Result<()>;

    /// Clear all cached text records.
    fn clear_text(&self) -> Result<()>;

    /// Get the latest image record for a user. If owner is None, returns the latest image record
    /// across all users (typically used by admin users).
    fn get_latest_image(&self, owner: Option<&str>) -> Result<Option<ImageRecord>>;

    /// Cache the latest image record for a user.
    fn save_latest_image(&self, owner: &str, image: ImageRecord) -> Result<()>;

    /// Remove the cached latest image record for a user.
    fn delete_latest_image(&self, owner: &str) -> Result<()>;

    /// Clear all cached image records.
    fn clear_image(&self) -> Result<()>;

    /// Get the latest file record for a user. If owner is None, returns the latest file record
    /// across all users (typically used by admin users).
    fn get_latest_file(&self, owner: Option<&str>) -> Result<Option<FileRecord>>;

    /// Cache the latest file record for a user.
    fn save_latest_file(&self, owner: &str, image: FileRecord) -> Result<()>;

    /// Remove the cached latest file record for a user.
    fn delete_latest_file(&self, owner: &str) -> Result<()>;

    /// Clear all cached file records.
    fn clear_file(&self) -> Result<()>;
}

/// A no-op cache implementation that doesn't cache any data.
/// All operations return empty results immediately.
#[derive(Debug, Clone, Copy)]
pub struct DisableCache;

/// Enum representing different supported cache types.
pub enum UnionCache {
    /// Memory-based cache implementation.
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
