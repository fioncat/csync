/// Converts a size in bytes to a human-readable string representation.
///
/// This function converts a byte size into a human-readable format using binary prefixes
/// (KiB, MiB, GiB, etc.). The result is rounded to one decimal place when necessary,
/// and the decimal is omitted for whole numbers.
///
/// # Arguments
/// * `bytes` - The size in bytes to convert. Can be any type that can be converted into u64.
///
/// # Returns
/// A string representing the size with appropriate binary unit suffix.
///
/// # Examples
/// ```
/// use crate::humanize::human_bytes;
///
/// assert_eq!(human_bytes(0), "0 B");
/// assert_eq!(human_bytes(1024), "1 KiB");
/// assert_eq!(human_bytes(1536), "1.5 KiB");
/// assert_eq!(human_bytes(1048576), "1 MiB");
/// ```
pub fn human_bytes(size: u64) -> String {
    const BYTES_UNIT: f64 = 1024.0;
    const BYTES_SUFFIX: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let size = size as f64;
    if size <= 0.0 {
        return String::from("0 B");
    }

    let base = size.log10() / BYTES_UNIT.log10();
    let result = format!("{:.1}", BYTES_UNIT.powf(base - base.floor()))
        .trim_end_matches(".0")
        .to_owned();

    [&result, BYTES_SUFFIX[base.floor() as usize]].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_bytes() {
        // Test zero and small values
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1), "1 B");
        assert_eq!(human_bytes(123), "123 B");

        // Test exact binary boundaries
        assert_eq!(human_bytes(1024), "1 KiB");
        assert_eq!(human_bytes(1024 * 1024), "1 MiB");
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1 GiB");

        // Test values with decimals
        assert_eq!(human_bytes(1536), "1.5 KiB"); // 1.5 KiB
        assert_eq!(human_bytes(2560), "2.5 KiB"); // 2.5 KiB
        assert_eq!(human_bytes(1536 * 1024), "1.5 MiB"); // 1.5 MiB

        // Test larger values
        assert_eq!(human_bytes(1024 * 1024 * 1024 * 1024), "1 TiB");

        // Test values just under boundaries
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1024 * 1024 - 1), "1024 KiB");

        // Test values just over boundaries
        assert_eq!(human_bytes(1025), "1 KiB");
        assert_eq!(human_bytes(1024 * 1024 + 1), "1 MiB");
    }
}
