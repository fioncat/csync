/// Convert a size to a human-readable string, for example, "32KB".
pub fn human_bytes<T: Into<u64>>(bytes: T) -> String {
    const BYTES_UNIT: f64 = 1024.0;
    const BYTES_SUFFIX: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let size = bytes.into();
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
