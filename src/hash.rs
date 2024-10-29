pub fn get_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    format!("{}", hash.to_hex())
}
