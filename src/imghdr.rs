#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ImageType {
    Png,
    Jpeg,
    Unknown,
}

pub fn detect_data_image_type(data: &[u8]) -> ImageType {
    if data.len() < 8 {
        return ImageType::Unknown;
    }

    const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    const JPEG_SIGNATURE: &[u8] = &[0xFF, 0xD8, 0xFF];

    if data.starts_with(PNG_SIGNATURE) {
        ImageType::Png
    } else if data.starts_with(JPEG_SIGNATURE) {
        ImageType::Jpeg
    } else {
        ImageType::Unknown
    }
}

pub fn is_data_image(data: &[u8]) -> bool {
    matches!(
        detect_data_image_type(data),
        ImageType::Png | ImageType::Jpeg
    )
}
