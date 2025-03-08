mod handler;
mod render;
mod state;

use anyhow::{Context, Result};
use csync_misc::config::PathSet;
use handler::TrayHandler;
use log::{error, info};
use state::TrayState;
use tauri::{AppHandle, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tokio::sync::mpsc;

use crate::remote::Remote;
use crate::tray::render::Renderer;

pub struct SystemTray {
    pub remote: Remote,
    pub ps: PathSet,
    pub copy_tx: mpsc::Sender<Vec<u8>>,
    pub state_rx: mpsc::Receiver<TrayState>,
}

impl SystemTray {
    pub fn new(
        remote: Remote,
        ps: PathSet,
        copy_tx: mpsc::Sender<Vec<u8>>,
        limit: u64,
        refresh_secs: u64,
    ) -> Self {
        let state_rx = TrayState::start(remote.clone(), limit, refresh_secs);
        Self {
            remote,
            ps,
            copy_tx,
            state_rx,
        }
    }

    #[allow(deprecated)]
    pub fn run(self) -> Result<()> {
        info!("Starting system tray event loop");

        let handler = TrayHandler::new(self.remote, self.ps, self.copy_tx);

        tauri::Builder::default()
            .setup(move |app| {
                // Hide the app icon from the dock(macOS) while keeping it in the menu bar
                // See: <https://github.com/tauri-apps/tauri/discussions/6038>
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);

                let app = app.handle().clone();
                tokio::spawn(async move {
                    refresh_menu(app, self.state_rx).await;
                });
                Ok(())
            })
            .on_menu_event(move |app, event| {
                let app = app.clone();
                let handler = handler.clone();
                tokio::spawn(async move {
                    let result = handler.handle_event(app.clone(), event.id.as_ref()).await;
                    match result {
                        Ok(None) => {}
                        Ok(Some(msg)) => dialog_info(app.clone(), msg),
                        Err(e) => {
                            let msg = format!("Error: {e:#}");
                            dialog_error(app, &msg);
                        }
                    }
                });
            })
            .on_window_event(|window, event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    if let Err(e) = window.hide() {
                        error!("Hide window error: {:#}", e);
                    }
                    api.prevent_close();
                }
            })
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_shell::init())
            .run(tauri::generate_context!())
            .context("run system tray event loop")
    }
}

async fn refresh_menu(app: AppHandle, mut state_rx: mpsc::Receiver<TrayState>) {
    info!("Begin to refresh tray menu");
    loop {
        let state = state_rx.recv().await.unwrap();
        if let Err(e) = render_menu(app.clone(), state) {
            error!("Failed to render tray menu: {e:#}");
        }
    }
}

fn render_menu(app: AppHandle, state: TrayState) -> Result<()> {
    let renderer = Renderer::new(app, state)?;
    renderer.render()?;
    Ok(())
}

fn dialog_info(app: AppHandle, msg: &str) {
    app.dialog()
        .message(msg)
        .kind(MessageDialogKind::Info)
        .title("Csync Information")
        .blocking_show();
}

fn dialog_error(app: AppHandle, msg: &str) {
    app.dialog()
        .message(msg)
        .kind(MessageDialogKind::Error)
        .title("Csync Error")
        .blocking_show();
}
