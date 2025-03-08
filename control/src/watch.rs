use anyhow::{bail, Result};
use async_trait::async_trait;
use clap::Args;
use csync_misc::api::metadata::{Event, EventType};
use csync_misc::client::config::ClientConfig;
use csync_misc::config::ConfigArgs;
use tokio::select;

use super::RunCommand;

/// Watch blob events from server
#[derive(Args)]
pub struct WatchArgs {
    #[command(flatten)]
    pub config: ConfigArgs,
}

#[async_trait]
impl RunCommand for WatchArgs {
    async fn run(&self) -> Result<()> {
        let cfg: ClientConfig = self.config.load("client")?;

        let mut events_sub = cfg.subscribe_events().await?;
        loop {
            select! {
                Some(event) = events_sub.events.recv() => {
                    Self::display_event(event);
                },

                Some(false) = events_sub.states.recv() => {
                    bail!("Listen event failed");
                }
            }
        }
    }
}

impl WatchArgs {
    fn display_event(event: Event) {
        let event_type = match event.event_type {
            EventType::Put => "PUT",
            EventType::Update => "UPDATE",
            EventType::Delete => "DELETE",
        };

        println!("<{event_type}>");
        for item in event.items {
            println!(
                "id={}, pin={}, owner={}, update_time={}, recycle_time={}, {}",
                item.id, item.pin, item.owner, item.update_time, item.recycle_time, item.summary
            );
        }
        println!();
    }
}
