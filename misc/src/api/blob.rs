use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{bail, Context, Result};

use crate::config::PathSet;
use crate::header::HeaderMap;
use crate::{code, dirs, parse_from_map};

use super::metadata::BlobType;
use super::{Request, RequestField, Value};

pub const BLOB_PATH: &str = "/v1/blob";

pub const HEADER_SHA256: &str = "X-Blob-Sha256";
pub const HEADER_BLOB_TYPE: &str = "X-Blob-Type";
pub const HEADER_FILE_NAME: &str = "X-File-Name";
pub const HEADER_FILE_MODE: &str = "X-File-Mode";

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Blob {
    pub data: Vec<u8>,
    pub sha256: String,

    pub blob_type: BlobType,

    pub file_name: Option<String>,
    pub file_mode: Option<u32>,
}

impl Blob {
    pub fn new_text(text: String) -> Self {
        let sha256 = code::sha256(&text);
        Self {
            data: text.into_bytes(),
            sha256,
            blob_type: BlobType::Text,
            ..Default::default()
        }
    }

    pub fn new_image(data: Vec<u8>) -> Self {
        let sha256 = code::sha256(&data);
        Self {
            data,
            sha256,
            blob_type: BlobType::Image,
            ..Default::default()
        }
    }

    pub fn new_file(data: Vec<u8>, file_name: String, file_mode: Option<u32>) -> Self {
        let sha256 = code::sha256(&data);
        Self {
            data,
            sha256,
            blob_type: BlobType::File,
            file_name: Some(file_name),
            file_mode,
        }
    }

    pub fn read_from_file(path: &Path) -> Result<Self> {
        let data = fs::read(path)?;

        let info = fs::metadata(path).context("get file metadata")?;
        let path = PathBuf::from(path);
        let name = path
            .file_name()
            .context("get file name")?
            .to_str()
            .unwrap_or_default();
        if name.is_empty() {
            bail!("empty file name");
        }
        let mode = Self::get_file_mode(info);

        Ok(Blob::new_file(data, name.to_string(), mode))
    }

    pub fn write_file(self, ps: &PathSet) -> Result<()> {
        let path = ps.data_dir.join("files");
        dirs::ensure_dir_exists(&path)?;

        self.write_file_to_dir(&path)?;

        println!("Saved file: {}", path.display());
        Ok(())
    }

    pub fn write_file_to_dir(self, dir: &Path) -> Result<PathBuf> {
        let name = self.file_name.unwrap_or_default();
        if name.is_empty() {
            bail!("file name is empty from server");
        }
        let path = dir.join(name);

        if let Some(mode) = self.file_mode {
            Self::write_file_with_mode(&path, &self.data, mode)?;
        } else {
            fs::write(&path, &self.data)?;
        }
        Ok(path)
    }

    pub fn write(self, ps: &PathSet) -> Result<()> {
        let mut stdout = io::stdout();
        let is_terminal = stdout.is_terminal();

        match self.blob_type {
            BlobType::Image => {
                if is_terminal {
                    bail!("cannot write image to terminal, please redirect stdout");
                }
                stdout.write_all(&self.data)?;
            }
            BlobType::Text => {
                let text = String::from_utf8(self.data)?;
                stdout.write_all(text.as_bytes())?;
            }
            BlobType::File => self.write_file(ps)?,
        }
        Ok(())
    }

    #[cfg(unix)]
    fn write_file_with_mode(path: &Path, data: &[u8], mode: u32) -> Result<()> {
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = fs::OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .mode(mode)
            .open(path)?;
        file.write_all(data)?;
        Ok(())
    }

    #[cfg(windows)]
    fn write_file_with_mode(path: &Path, data: &[u8], _mode: u32) -> Result<()> {
        fs::write(path, data)?;
        Ok(())
    }

    #[cfg(unix)]
    fn get_file_mode(info: std::fs::Metadata) -> Option<u32> {
        use std::os::unix::fs::MetadataExt;
        Some(info.mode())
    }

    #[cfg(windows)]
    fn get_file_mode(_info: std::fs::Metadata) -> Option<u32> {
        None
    }
}

impl Request for Blob {
    fn is_data(&self) -> bool {
        true
    }

    fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    fn data(self) -> Vec<u8> {
        self.data
    }

    fn append_headers(&self, headers: &mut HashMap<&str, String>) {
        headers.insert(HEADER_SHA256, self.sha256.clone());
        headers.insert(HEADER_BLOB_TYPE, self.blob_type.to_string());
        if let Some(ref file_name) = self.file_name {
            headers.insert(HEADER_FILE_NAME, file_name.clone());
        }
        if let Some(file_mode) = self.file_mode {
            headers.insert(HEADER_FILE_MODE, file_mode.to_string());
        }
    }

    fn complete_headers(&mut self, mut headers: HeaderMap) -> Result<()> {
        self.sha256 = headers.get(HEADER_SHA256).cloned().unwrap_or_default();
        if self.sha256.is_empty() {
            bail!("sha256 for blob is required");
        }

        self.blob_type = match parse_from_map!(headers, HEADER_BLOB_TYPE) {
            Some(blob_type) => blob_type,
            None => bail!("blob type is required"),
        };

        self.file_name = headers.remove(HEADER_FILE_NAME);
        if matches!(self.blob_type, BlobType::File) && self.file_name.is_none() {
            bail!("file name is required for file blob");
        }

        self.file_mode = parse_from_map!(headers, HEADER_FILE_MODE);

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct GetBlobRequest {
    pub id: u64,
}

impl Request for GetBlobRequest {
    fn fields(self) -> Vec<RequestField> {
        vec![RequestField {
            name: "id",
            value: Value::Integer(self.id),
        }]
    }

    fn complete(&mut self, fields: HashMap<String, String>) -> Result<()> {
        self.id = parse_from_map!(fields, "id").unwrap_or_default();
        if self.id == 0 {
            bail!("id is required to get blob");
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct PatchBlobRequest {
    pub id: u64,

    pub pin: Option<bool>,
}

impl Request for PatchBlobRequest {
    fn fields(self) -> Vec<RequestField> {
        let mut fields = vec![RequestField {
            name: "id",
            value: Value::Integer(self.id),
        }];
        if let Some(pin) = self.pin {
            fields.push(RequestField {
                name: "pin",
                value: Value::Text(pin.to_string()),
            });
        }
        fields
    }

    fn complete(&mut self, fields: HashMap<String, String>) -> Result<()> {
        self.id = parse_from_map!(fields, "id").unwrap_or_default();
        if self.id == 0 {
            bail!("id is required to patch blob");
        }

        self.pin = parse_from_map!(fields, "pin");

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct DeleteBlobRequest {
    pub id: u64,
}

impl Request for DeleteBlobRequest {
    fn fields(self) -> Vec<RequestField> {
        vec![RequestField {
            name: "id",
            value: Value::Integer(self.id),
        }]
    }

    fn complete(&mut self, fields: HashMap<String, String>) -> Result<()> {
        self.id = parse_from_map!(fields, "id").unwrap_or_default();
        if self.id == 0 {
            bail!("id is required to delete blob");
        }
        Ok(())
    }
}
