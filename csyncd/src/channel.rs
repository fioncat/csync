use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use csync_proto::frame::Frame;
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

struct ChannelManager {
    channels: HashMap<String, Channel>,

    register: mpsc::Receiver<RegisterRequest>,
    push: mpsc::Receiver<PushRequest>,
    pull: mpsc::Receiver<PullRequest>,
    close: mpsc::Receiver<CloseRequest>,
}

#[derive(Clone)]
pub struct ChannelHandler {
    register: mpsc::Sender<RegisterRequest>,
    push: mpsc::Sender<PushRequest>,
    pull: mpsc::Sender<PullRequest>,
    close: mpsc::Sender<CloseRequest>,
}

struct RegisterRequest {
    publish: Arc<String>,

    resp: Option<oneshot::Sender<()>>,
}

struct PushRequest {
    publish: Arc<String>,
    data: Frame,

    resp: Option<oneshot::Sender<()>>,
}

struct PullRequest {
    addr: Arc<String>,
    subs: Arc<Vec<String>>,

    resp: Option<oneshot::Sender<Option<Arc<Frame>>>>,
}

struct CloseRequest {
    addr: Arc<String>,

    publish: Option<Arc<String>>,
    subs: Option<Arc<Vec<String>>>,

    resp: Option<oneshot::Sender<()>>,
}

#[derive(Debug)]
struct Channel {
    data: Option<Arc<Frame>>,
    subs: HashMap<String, bool>,
    count: u64,
}

impl ChannelHandler {
    const CHANNEL_BUFFER_SIZE: usize = 4096;

    pub async fn new() -> ChannelHandler {
        let (register_tx, register_rx) =
            mpsc::channel::<RegisterRequest>(Self::CHANNEL_BUFFER_SIZE);
        let (push_tx, push_rx) = mpsc::channel::<PushRequest>(Self::CHANNEL_BUFFER_SIZE);
        let (pull_tx, pull_rx) = mpsc::channel::<PullRequest>(Self::CHANNEL_BUFFER_SIZE);
        let (close_tx, close_rx) = mpsc::channel::<CloseRequest>(Self::CHANNEL_BUFFER_SIZE);

        let mut mgr = ChannelManager {
            channels: HashMap::new(),
            register: register_rx,
            push: push_rx,
            pull: pull_rx,
            close: close_rx,
        };
        tokio::spawn(async move {
            mgr.main_loop().await;
        });

        ChannelHandler {
            register: register_tx,
            push: push_tx,
            pull: pull_tx,
            close: close_tx,
        }
    }

    pub async fn register(&self, publish: Arc<String>) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel::<()>();
        let req = RegisterRequest {
            publish,
            resp: Some(resp_tx),
        };
        self.register
            .send(req)
            .await
            .context("send register request to channel")?;
        resp_rx
            .await
            .context("recv register response from channel")?;
        Ok(())
    }

    pub async fn push(&self, publish: Arc<String>, data: Frame) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel::<()>();
        let req = PushRequest {
            publish,
            data,
            resp: Some(resp_tx),
        };
        self.push
            .send(req)
            .await
            .context("send push request to channel")?;
        resp_rx.await.context("recv push response from channel")?;
        Ok(())
    }

    pub async fn pull(
        &self,
        addr: Arc<String>,
        subs: Arc<Vec<String>>,
    ) -> Result<Option<Arc<Frame>>> {
        let (resp_tx, resp_rx) = oneshot::channel::<Option<Arc<Frame>>>();
        let req = PullRequest {
            addr,
            subs,
            resp: Some(resp_tx),
        };
        self.pull
            .send(req)
            .await
            .context("send pull request to channel")?;
        let frame = resp_rx.await.context("recv pull response from channel")?;
        Ok(frame)
    }

    pub async fn close(
        &self,
        addr: Arc<String>,
        publish: Option<Arc<String>>,
        subs: Option<Arc<Vec<String>>>,
    ) -> Result<()> {
        if let None = publish {
            if let None = subs {
                return Ok(());
            }
        }
        let (resp_tx, resp_rx) = oneshot::channel::<()>();
        let req = CloseRequest {
            addr,
            publish,
            subs,
            resp: Some(resp_tx),
        };
        self.close
            .send(req)
            .await
            .context("send close request to channel")?;
        resp_rx.await.context("recv close response from channel")?;
        Ok(())
    }
}

impl ChannelManager {
    async fn main_loop(&mut self) {
        loop {
            select! {
                Some(mut req) = self.register.recv() => {
                    let resp = req.resp.take().unwrap();
                    self.handle_register(req);
                    resp.send(()).unwrap();
                }
                Some(mut req) = self.push.recv() => {
                    let resp = req.resp.take().unwrap();
                    self.handle_push(req);
                    resp.send(()).unwrap();
                }
                Some(mut req) = self.pull.recv() => {
                    let resp = req.resp.take().unwrap();
                    let data = self.handle_pull(req);
                    resp.send(data).unwrap();
                }
                Some(mut req) = self.close.recv() => {
                    let resp = req.resp.take().unwrap();
                    self.handle_close(req);
                    resp.send(()).unwrap();
                }
            }
        }
    }

    fn handle_register(&mut self, req: RegisterRequest) {
        let (publish, mut channel) = self.channels.remove_entry(req.publish.as_ref()).unwrap_or((
            req.publish.as_ref().to_string(),
            Channel {
                data: None,
                subs: HashMap::new(),
                count: 0,
            },
        ));
        channel.count += 1;
        self.channels.insert(publish, channel);
    }

    fn handle_push(&mut self, req: PushRequest) {
        let channel = match self.channels.get_mut(req.publish.as_ref()) {
            Some(channel) => channel,
            None => return,
        };

        channel.data = Some(Arc::new(req.data));
        for (_, update) in channel.subs.iter_mut() {
            *update = true;
        }
    }

    fn handle_pull(&mut self, req: PullRequest) -> Option<Arc<Frame>> {
        let mut result = None;
        for sub in req.subs.iter() {
            let channel = match self.channels.get_mut(sub) {
                Some(channel) => channel,
                None => continue,
            };
            if let None = channel.data {
                continue;
            }
            if let Some(update) = channel.subs.get_mut(req.addr.as_ref()) {
                if !*update {
                    continue;
                }
                *update = false;
            } else {
                channel.subs.insert(req.addr.as_ref().to_string(), false);
            }

            if let None = result {
                let frame = channel.data.as_ref().unwrap();
                let frame = Arc::clone(frame);
                result = Some(frame);
            }
        }
        result
    }

    fn handle_close(&mut self, req: CloseRequest) {
        let CloseRequest {
            addr,
            publish,
            subs,
            resp: _,
        } = req;

        if let Some(publish) = publish.as_ref() {
            if let Some((publish, mut channel)) = self.channels.remove_entry(publish.as_ref()) {
                channel.count -= 1;
                if channel.count > 0 {
                    self.channels.insert(publish, channel);
                }
            }
        }

        if let Some(subs) = subs {
            for sub in subs.iter() {
                if let Some(channel) = self.channels.get_mut(sub) {
                    channel.subs.remove(addr.as_ref());
                }
            }
        }

        if cfg!(test) {
            self.assert_addr_remove(addr.as_ref());
            if let Some(publish) = publish.as_ref() {
                if let Some(_) = self.channels.get(publish.as_ref()) {
                    panic!("unexpect channel exists: {publish}");
                }
            }
        }
    }

    fn assert_addr_remove(&self, addr: &str) {
        for (_, channel) in self.channels.iter() {
            assert_eq!(channel.subs.get(addr), None);
        }
    }
}
