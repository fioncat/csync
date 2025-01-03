pub struct StoreConfig {
    pub data_path: String,
    pub name: String,
    pub cache: StoreCacheConfig,
}

pub struct StoreCacheConfig {
    pub enable: bool,
    pub expiry: usize,
}
