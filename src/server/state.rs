use std::sync::Arc;

use tokio::select;
use tokio::sync::{mpsc, oneshot};

use crate::types::{Data, Header};

const CHANNEL_BUFFER_SIZE: usize = 512;

#[derive(Debug, PartialEq)]
pub enum State {
    UpToDate,
    Outdated(Arc<Data>),
    FastForward,
}

pub struct StateManager {
    get_tx: mpsc::Sender<GetStateRequest>,
    update_tx: mpsc::Sender<UpdateStateRequest>,
}

impl StateManager {
    pub async fn new() -> Self {
        let (get_tx, get_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (update_tx, update_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        let mut handler = StateHandler {
            state: None,
            get_rx,
            update_rx,
        };
        tokio::spawn(async move {
            handler.main_loop().await;
        });

        Self { get_tx, update_tx }
    }

    pub async fn get(&self, header: Header) -> State {
        let (resp_tx, resp_rx) = oneshot::channel::<State>();
        let req = GetStateRequest {
            header,
            resp: Some(resp_tx),
        };

        self.get_tx.send(req).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn update(&self, data: Data) -> u64 {
        let (resp_tx, resp_rx) = oneshot::channel::<u64>();
        let req = UpdateStateRequest {
            data,
            resp: Some(resp_tx),
        };

        self.update_tx.send(req).await.unwrap();
        resp_rx.await.unwrap()
    }
}

struct GetStateRequest {
    header: Header,
    resp: Option<oneshot::Sender<State>>,
}

struct UpdateStateRequest {
    data: Data,
    resp: Option<oneshot::Sender<u64>>,
}

struct StateHandler {
    state: Option<Arc<Data>>,

    get_rx: mpsc::Receiver<GetStateRequest>,
    update_rx: mpsc::Receiver<UpdateStateRequest>,
}

impl StateHandler {
    async fn main_loop(&mut self) {
        loop {
            select! {
                Some(mut req) = self.get_rx.recv() => {
                    let resp = req.resp.take().unwrap();
                    let state = self.handle_get(req.header);
                    let _ = resp.send(state);
                },
                Some(mut req) = self.update_rx.recv() => {
                    let resp = req.resp.take().unwrap();
                    let revision = self.handle_update(req.data);
                    let _ = resp.send(revision);
                },
            }
        }
    }

    fn handle_get(&self, header: Header) -> State {
        if self.state.is_none() {
            return State::FastForward;
        }
        let current_data = self.state.as_ref().unwrap();
        if header.revision < current_data.header.revision {
            return State::Outdated(Arc::clone(current_data));
        }

        // TODO: when the request revision is higher than the state, what should we do?
        // Maybe we should return FastForward to update current state with the latest data.

        if header.digest == current_data.header.digest {
            return State::UpToDate;
        }

        State::FastForward
    }

    fn handle_update(&mut self, mut data: Data) -> u64 {
        let revision = self
            .state
            .as_ref()
            .map(|data| data.header.revision + 1)
            .unwrap_or(0);
        data.header.revision = revision;
        self.state = Some(Arc::new(data));
        revision
    }
}

#[cfg(test)]
mod test_state {
    use super::*;

    #[tokio::test]
    async fn test_state_manager() {
        let state_manager = StateManager::new().await;

        let state = state_manager
            .get(Header {
                digest: String::from("digest0"),
                revision: 0,
            })
            .await;
        // When the state is empty, the first get request should return FastForward.
        assert_eq!(state, State::FastForward);

        let state = state_manager
            .get(Header {
                digest: String::from("digest1"),
                revision: 0,
            })
            .await;
        // The state is still empty
        assert_eq!(state, State::FastForward);

        let revision = state_manager
            .update(Data {
                header: Header {
                    digest: String::from("digest3"),
                    revision: 0,
                },
                bytes: b"hello".to_vec(),
            })
            .await;
        assert_eq!(revision, 0);

        let state = state_manager
            .get(Header {
                digest: String::from("digest4"),
                revision: 0,
            })
            .await;
        assert_eq!(state, State::FastForward);

        let revision = state_manager
            .update(Data {
                header: Header {
                    digest: String::from("digest4"),
                    revision: 0,
                },
                bytes: b"world".to_vec(),
            })
            .await;
        assert_eq!(revision, 1);

        let state = state_manager
            .get(Header {
                digest: String::from("digest5"),
                revision: 0,
            })
            .await;
        // The revision is less than state, the outdated state should be returned.
        if let State::Outdated(data) = state {
            assert_eq!(
                data.as_ref(),
                &Data {
                    header: Header {
                        digest: String::from("digest4"),
                        revision: 1,
                    },
                    bytes: b"world".to_vec(),
                }
            );
        } else {
            panic!("expect outdated state, found: {state:?}");
        }

        let state = state_manager
            .get(Header {
                digest: String::from("digest4"),
                revision: 1,
            })
            .await;
        assert_eq!(state, State::UpToDate);
    }
}
