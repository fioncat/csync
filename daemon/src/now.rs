#[cfg(test)]
use std::sync::Mutex;

use chrono::Local;

#[cfg(test)]
static MOCK_TIME: once_cell::sync::Lazy<Mutex<u64>> =
    once_cell::sync::Lazy::new(|| Mutex::new(Local::now().timestamp() as u64));

#[cfg(test)]
pub fn advance_mock_time(seconds: u64) {
    let mut guard = MOCK_TIME.lock().unwrap();
    *guard += seconds;
}

#[cfg(test)]
pub fn current_timestamp() -> u64 {
    *MOCK_TIME.lock().unwrap()
}

#[cfg(not(test))]
pub fn current_timestamp() -> u64 {
    Local::now().timestamp() as u64
}
