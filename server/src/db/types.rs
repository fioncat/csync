use anyhow::Result;
use csync_misc::api::blob::{Blob, PatchBlobRequest};
use csync_misc::api::metadata::{GetMetadataRequest, Metadata};
use csync_misc::api::user::{GetUserRequest, PatchUserRequest, PutUserRequest, User};

pub trait Connection<'a, T>
where
    T: Transaction + 'a,
{
    fn transaction(&'a mut self) -> Result<T>;
}

pub trait Transaction {
    fn create_blob(&self, params: CreateBlobParams) -> Result<u64>;
    fn update_blob(&self, params: PatchBlobParams) -> Result<()>;
    fn delete_blob(&self, id: u64) -> Result<()>;
    fn delete_blobs(&self, ids: Vec<u64>) -> Result<u64>;
    fn get_blob(&self, id: u64) -> Result<Blob>;
    fn has_blob(&self, id: u64) -> Result<bool>;

    fn get_metadata(&self, id: u64) -> Result<Metadata>;
    fn count_metadatas(&self, req: GetMetadataRequest) -> Result<u64>;
    fn get_metadatas(&self, req: GetMetadataRequest) -> Result<Vec<Metadata>>;

    fn create_user(&self, params: CreateUserParams) -> Result<()>;
    fn update_user(&self, patch: PatchUserRequest, update_time: u64) -> Result<()>;
    fn delete_user(&self, name: &str) -> Result<()>;
    fn has_user(&self, name: String) -> Result<bool>;
    fn get_user_password(&self, name: String) -> Result<UserPassword>;
    fn count_users(&self, req: GetUserRequest) -> Result<u64>;
    fn get_users(&self, req: GetUserRequest) -> Result<Vec<User>>;

    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct CreateBlobParams {
    pub blob: Blob,
    pub summary: String,
    pub owner: String,
    pub update_time: u64,
    pub recycle_time: u64,
}

#[derive(Debug, Default)]
pub struct PatchBlobParams {
    pub patch: PatchBlobRequest,
    pub update_time: u64,
    pub recycle_time: u64,
}

#[derive(Debug, Default)]
pub struct CreateUserParams {
    pub user: PutUserRequest,
    pub salt: String,
    pub update_time: u64,
}

#[derive(Debug, Default, PartialEq)]
pub struct UserPassword {
    pub name: String,
    pub password: String,
    pub salt: String,
    pub admin: bool,
}
