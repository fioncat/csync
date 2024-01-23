mod clipboard;

use csync_proto::frame::ClipboardFrame;

pub use clipboard::Clipboard;
use sha256::digest;

pub fn get_digest(data: &ClipboardFrame) -> String {
    match data {
        ClipboardFrame::Text(text) => digest(text),
        ClipboardFrame::Image(image) => digest::<&[u8]>(&image.data),
    }
}
