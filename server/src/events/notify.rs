use std::collections::HashMap;

use csync_misc::api::metadata::Event;
use log::{debug, info, warn};
use tokio::select;
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Clone)]
pub struct Notifier {
    subs_tx: mpsc::Sender<SubscribeRequest>,
}

impl Notifier {
    pub fn start(events_rx: mpsc::Receiver<Event>) -> Self {
        let (subs_tx, subs_rx) = mpsc::channel(100);

        let mut dispatcher = Dispatcher {
            events_rx,
            subs_rx,
            subs: HashMap::new(),
        };
        tokio::spawn(async move {
            dispatcher.main_loop().await;
        });

        Self { subs_tx }
    }

    pub async fn subscribe(&self, name: String) -> broadcast::Receiver<Event> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let req = SubscribeRequest { name, resp_tx };
        self.subs_tx.send(req).await.unwrap();

        resp_rx.await.unwrap()
    }
}

struct SubscribeRequest {
    name: String,
    resp_tx: oneshot::Sender<broadcast::Receiver<Event>>,
}

struct Dispatcher {
    events_rx: mpsc::Receiver<Event>,
    subs_rx: mpsc::Receiver<SubscribeRequest>,

    subs: HashMap<String, broadcast::Sender<Event>>,
}

impl Dispatcher {
    async fn main_loop(&mut self) {
        info!("Start events dispatcher main loop");
        loop {
            select! {
                Some(sub_req) = self.subs_rx.recv() => {
                    self.handle_sub(sub_req).await;
                },

                Some(event) = self.events_rx.recv() => {
                    self.handle_event(event).await;
                },
            }
        }
    }

    async fn handle_sub(&mut self, req: SubscribeRequest) {
        debug!("Allocate new events subscription for user {}", req.name);
        let resp = req.resp_tx;
        if let Some(sub_tx) = self.subs.get(&req.name) {
            let sub_rx = sub_tx.subscribe();
            resp.send(sub_rx).unwrap();
            return;
        }

        let (sub_tx, sub_rx) = broadcast::channel(500);
        self.subs.insert(req.name, sub_tx);
        resp.send(sub_rx).unwrap();

        debug!("Current subscriptions: {:?}", self.subs.keys());
    }

    async fn handle_event(&mut self, event: Event) {
        let mut user_events: HashMap<String, Event> = HashMap::new();
        for item in event.items {
            match user_events.get_mut(&item.owner) {
                Some(event) => event.items.push(item),
                None => {
                    let owner = item.owner.clone();
                    let user_event = Event {
                        event_type: event.event_type,
                        items: vec![item],
                    };
                    user_events.insert(owner, user_event);
                }
            }
        }
        debug!("Received event, group by user: {:?}", user_events);
        if user_events.is_empty() {
            warn!("Received event with no items");
            return;
        }

        for (user, event) in user_events {
            let need_remove = match self.subs.get(&user) {
                Some(sub_tx) => {
                    if sub_tx.receiver_count() == 0 {
                        info!("No more subscribers for user {}, remove subscription", user);
                        true
                    } else {
                        debug!("Send event to user {}: {:?}", user, event);
                        sub_tx.send(event).unwrap();
                        false
                    }
                }
                None => {
                    debug!(
                        "User {} has no subscription, skip sending event {:?}",
                        user, event
                    );
                    false
                }
            };
            if need_remove {
                self.subs.remove(&user);
            }
        }

        debug!("Current subscriptions: {:?}", self.subs.keys());
    }
}
