use anyhow::Result;

pub fn check() -> Result<()> {
    Ok(())
}

pub fn read_text() -> Result<Option<String>> {
    Ok(None)
}

pub fn write_text(_text: String) -> Result<()> {
    Ok(())
}

pub fn read_image() -> Result<Option<Vec<u8>>> {
    Ok(None)
}

pub fn write_image(_data: Vec<u8>) -> Result<()> {
    Ok(())
}

pub fn is_image() -> Result<bool> {
    Ok(false)
}

pub fn is_text() -> Result<bool> {
    Ok(false)
}
