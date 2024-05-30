#[derive(Debug, PartialEq)]
pub struct Data {
    pub header: Header,
    pub bytes: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct Header {
    pub digest: String,
    pub revision: u64,
}
