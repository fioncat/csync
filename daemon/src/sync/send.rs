use csync_misc::imghdr::is_data_image;
use tokio::sync::mpsc;

#[derive(Clone, Debug, Default)]
pub struct SyncSender {
    pub text_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub image_tx: Option<mpsc::Sender<Vec<u8>>>,
}

impl SyncSender {
    pub async fn send(&mut self, data: Vec<u8>) {
        if is_data_image(&data) {
            if let Some(ref tx) = self.image_tx {
                tx.send(data).await.unwrap();
            }
            return;
        }

        if let Some(ref tx) = self.text_tx {
            tx.send(data).await.unwrap();
        }
    }
}
