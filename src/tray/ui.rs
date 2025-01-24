use std::sync::Arc;

use anyhow::{Context, Result};
use log::{error, info};
use tauri::{AppHandle, SystemTray, SystemTrayEvent, SystemTrayMenu};
use tokio::sync::{mpsc, Mutex};

pub async fn build_and_run_tray_ui(
    default_menu: SystemTrayMenu,
    menu_rx: mpsc::Receiver<SystemTrayMenu>,
    write_tx: mpsc::Sender<u64>,
) -> Result<()> {
    let system_tray = SystemTray::new().with_menu(default_menu);

    let write_tx = Arc::new(Mutex::new(write_tx));

    info!("Starting system tray event loop");
    tauri::Builder::default()
        .system_tray(system_tray)
        .setup(|app| {
            // Hide the app icon from the dock(macOS) while keeping it in the menu bar
            // See: <https://github.com/tauri-apps/tauri/discussions/6038>
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_handle = app.handle();
            tokio::spawn(async move {
                watch_menu_updates(app_handle, menu_rx).await;
            });
            Ok(())
        })
        .on_system_tray_event(move |_app, event| {
            if let SystemTrayEvent::MenuItemClick { id, .. } = event {
                let id = match id.parse::<u64>() {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Failed to parse menu item ID: {e:#}");
                        return;
                    }
                };
                let write_tx = write_tx.clone();
                tokio::spawn(async move {
                    info!("Sending menu item click event: {id}");
                    write_tx.lock().await.send(id).await.unwrap();
                });
            }
        })
        .run(tauri::generate_context!())
        .context("run system tray event loop")
}

async fn watch_menu_updates(app_handle: AppHandle, mut menu_rx: mpsc::Receiver<SystemTrayMenu>) {
    info!("Watching menu updates");
    loop {
        let menu = menu_rx.recv().await.unwrap();
        info!("Received menu update event");
        let tray_handle = app_handle.tray_handle();
        if let Err(e) = tray_handle.set_menu(menu) {
            error!("Failed to update system tray menu: {e:#}");
        }
    }
}
