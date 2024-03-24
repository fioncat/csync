use gethostname::gethostname;

pub(super) fn server() -> String {
    String::new()
}

pub(super) fn device() -> String {
    let hostname = gethostname();
    if let Some(hostname) = hostname.to_str() {
        if !hostname.is_empty() {
            return String::from(hostname);
        }
    }

    String::from("default-device")
}

pub(super) fn empty_vec() -> Vec<String> {
    Vec::new()
}

pub(super) fn work_dir() -> String {
    // TODO: Support Windows?
    String::from("~/Downloads/csync")
}

pub(super) fn read_interval() -> u32 {
    300
}

pub(super) fn disable() -> bool {
    false
}
