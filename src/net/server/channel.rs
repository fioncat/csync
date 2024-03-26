use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use log::{error, info};
use tokio::select;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::net::frame::DataFrame;

const CHANNEL_BUFFER_SIZE: usize = 512;

pub struct DataReceiver(mpsc::Receiver<Arc<DataFrame>>);

impl Deref for DataReceiver {
    type Target = mpsc::Receiver<Arc<DataFrame>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DataReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
pub struct ChannelRequest {
    sub_tx: mpsc::Sender<SubRequest>,
    pub_tx: mpsc::Sender<PubRequest>,
    close_tx: mpsc::Sender<CloseRequest>,
}

impl ChannelRequest {
    pub async fn new() -> ChannelRequest {
        let (sub_tx, sub_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (pub_tx, pub_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (close_tx, close_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let mut channel = Channel {
            subs: HashMap::new(),
            sub_rx,
            pub_rx,
            close_rx,
        };
        tokio::spawn(async move {
            channel.main_loop().await;
        });

        ChannelRequest {
            sub_tx,
            pub_tx,
            close_tx,
        }
    }

    pub async fn subscribe(&self, devices: Arc<Vec<String>>) -> (String, DataReceiver) {
        let (resp_tx, resp_rx) = oneshot::channel::<(String, DataReceiver)>();
        let req = SubRequest {
            devices,
            resp: Some(resp_tx),
        };

        self.sub_tx.send(req).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn publish(&self, device: Arc<String>, data: DataFrame) {
        let (resp_tx, resp_rx) = oneshot::channel::<()>();
        let req = PubRequest {
            data,
            device,
            resp: Some(resp_tx),
        };

        self.pub_tx.send(req).await.unwrap();
        resp_rx.await.unwrap();
    }

    pub async fn close(&self, uuid: String) {
        let (resp_tx, resp_rx) = oneshot::channel::<()>();
        let req = CloseRequest {
            uuid,
            resp: Some(resp_tx),
        };

        self.close_tx.send(req).await.unwrap();
        resp_rx.await.unwrap();
    }
}

struct DataSender(mpsc::Sender<Arc<DataFrame>>);

struct DataSenders(HashMap<Arc<String>, DataSender>);

struct Channel {
    subs: HashMap<String, DataSenders>,

    sub_rx: mpsc::Receiver<SubRequest>,
    pub_rx: mpsc::Receiver<PubRequest>,
    close_rx: mpsc::Receiver<CloseRequest>,
}

struct SubRequest {
    devices: Arc<Vec<String>>,

    resp: Option<oneshot::Sender<(String, DataReceiver)>>,
}

struct PubRequest {
    device: Arc<String>,
    data: DataFrame,

    resp: Option<oneshot::Sender<()>>,
}

struct CloseRequest {
    uuid: String,

    resp: Option<oneshot::Sender<()>>,
}

impl Channel {
    async fn main_loop(&mut self) {
        loop {
            select! {
                Some(mut req) = self.sub_rx.recv() => {
                    let resp = req.resp.take().unwrap();
                    let (uuid, rx) = self.handle_sub(req);
                    let _ = resp.send((uuid, rx));
                },

                Some(mut req) = self.pub_rx.recv() => {
                    let resp = req.resp.take().unwrap();
                    self.handle_pub(req).await;
                    resp.send(()).unwrap();
                },

                Some(mut req) = self.close_rx.recv() => {
                    let resp = req.resp.take().unwrap();
                    self.handle_close(req);
                    resp.send(()).unwrap();
                },
            }
        }
    }

    fn handle_sub(&mut self, req: SubRequest) -> (String, DataReceiver) {
        let uuid = Uuid::new_v4().to_string();
        let uuid_rc = Arc::new(uuid.clone());

        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        for device in req.devices.iter() {
            let (device, mut data_senders) = self
                .subs
                .remove_entry(device)
                .unwrap_or((device.to_string(), DataSenders(HashMap::with_capacity(1))));

            data_senders
                .0
                .insert(Arc::clone(&uuid_rc), DataSender(tx.clone()));

            info!("[Channel] Add new sub '{uuid}' for device '{device}'");
            self.subs.insert(device, data_senders);
        }

        self.report_subs();
        (uuid, DataReceiver(rx))
    }

    async fn handle_pub(&mut self, req: PubRequest) {
        let data_senders = match self.subs.get_mut(req.device.as_ref()) {
            Some(ds) => ds,
            None => return,
        };

        let PubRequest {
            device,
            data: data_frame,
            resp: _,
        } = req;
        let data_frame = Arc::new(data_frame);
        for (uuid, sender) in data_senders.0.iter_mut() {
            info!("[Channel] Send data from device '{device}' to sub '{uuid}'");
            let result = sender.0.send(Arc::clone(&data_frame)).await;
            if let Err(err) = result {
                error!("[Channel] Inner error: send data to sub channel for device '{device}' unexpectly failed: {err}");
            }
        }
    }

    fn handle_close(&mut self, req: CloseRequest) {
        let mut to_remove = Vec::new();

        for (device, data_senders) in self.subs.iter_mut() {
            info!("[Channel] Remove sub '{}' for device '{device}'", req.uuid);
            data_senders.0.remove(&req.uuid);
            if data_senders.0.is_empty() {
                info!("[Channel] The device '{device}' subs is empty, remove it in channel");
                to_remove.push(device.clone());
            }
        }

        for device in to_remove {
            self.subs.remove(&device);
        }

        self.report_subs();
    }

    fn report_subs(&self) {
        let mut sum = HashMap::with_capacity(self.subs.len());
        for (device, data_senders) in self.subs.iter() {
            sum.insert(device, data_senders.0.len());
        }

        info!("[Channel] Updated channel count: {:?}", sum);
    }
}
