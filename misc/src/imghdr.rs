/// Represents the type of an image file.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ImageType {
    /// PNG image format
    Png,
    /// JPEG image format
    Jpeg,
    /// Unknown or unsupported image format
    Unknown,
}

/// Detects the image type by examining the file signature (magic numbers) in the data.
///
/// # Arguments
/// * `data` - Raw bytes of the image file
///
/// # Returns
/// The detected [`ImageType`]
///
/// # Examples
/// ```
/// use crate::imghdr::{detect_data_image_type, ImageType};
///
/// // PNG signature
/// let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
/// assert_eq!(detect_data_image_type(&png_data), ImageType::Png);
///
/// // JPEG signature
/// let jpeg_data = [0xFF, 0xD8, 0xFF, 0x00];
/// assert_eq!(detect_data_image_type(&jpeg_data), ImageType::Jpeg);
/// ```
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

/// Checks if the provided data represents a supported image format (PNG or JPEG).
///
/// # Arguments
/// * `data` - Raw bytes of the file to check
///
/// # Returns
/// `true` if the data represents a PNG or JPEG image, `false` otherwise
///
/// # Examples
/// ```
/// use crate::imghdr::is_data_image;
///
/// // PNG data
/// let png_data = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
/// assert!(is_data_image(&png_data));
///
/// // Not an image
/// let invalid_data = [0x00, 0x01, 0x02, 0x03];
/// assert!(!is_data_image(&invalid_data));
/// ```
pub fn is_data_image(data: &[u8]) -> bool {
    matches!(
        detect_data_image_type(data),
        ImageType::Png | ImageType::Jpeg
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imghdr() {
        // Test PNG detection
        let png_data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00,
            0x0D, // Additional PNG data
        ];
        assert_eq!(detect_data_image_type(&png_data), ImageType::Png);
        assert!(is_data_image(&png_data));

        // Test JPEG detection
        let jpeg_data = [
            0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49,
            0x46, // JPEG data with JFIF marker
        ];
        assert_eq!(detect_data_image_type(&jpeg_data), ImageType::Jpeg);
        assert!(is_data_image(&jpeg_data));

        // Test unknown/invalid cases
        assert_eq!(detect_data_image_type(&[]), ImageType::Unknown);
        assert_eq!(detect_data_image_type(&[0x89, 0x50]), ImageType::Unknown);

        let invalid_data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(detect_data_image_type(&invalid_data), ImageType::Unknown);
        assert!(!is_data_image(&invalid_data));
        assert!(!is_data_image(&[]));

        // Test ImageType traits
        assert_eq!(format!("{:?}", ImageType::Png), "Png");
        let t1 = ImageType::Png;
        let t2 = t1;
        assert_eq!(t1, t2);
        assert_ne!(ImageType::Png, ImageType::Jpeg);
    }
}
