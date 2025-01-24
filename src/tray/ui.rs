#![allow(deprecated)]

use std::sync::Arc;

use anyhow::{Context, Result};
use log::{error, info};
use tauri::menu::{Menu, MenuItem};
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};

pub async fn build_and_run_tray_ui(
    default_menu: Vec<(String, String)>,
    menu_rx: mpsc::Receiver<Vec<(String, String)>>,
    write_tx: mpsc::Sender<u64>,
) -> Result<()> {
    let write_tx = Arc::new(Mutex::new(write_tx));

    info!("Starting system tray event loop");
    tauri::Builder::default()
        .setup(|app| {
            // Hide the app icon from the dock(macOS) while keeping it in the menu bar
            // See: <https://github.com/tauri-apps/tauri/discussions/6038>
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            setup_menu(app.handle(), default_menu)?;

            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                watch_menu_updates(app_handle, menu_rx).await;
            });
            Ok(())
        })
        .on_menu_event(move |_app, event| {
            let id = match event.id.as_ref().parse::<u64>() {
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
        })
        .run(tauri::generate_context!())
        .context("run system tray event loop")
}

async fn watch_menu_updates(
    app_handle: AppHandle,
    mut menu_rx: mpsc::Receiver<Vec<(String, String)>>,
) {
    info!("Watching menu updates");
    loop {
        let items = menu_rx.recv().await.unwrap();
        info!("Received menu update event");

        if let Err(e) = setup_menu(&app_handle, items) {
            error!("Failed to update system tray menu: {e:#}");
        }
    }
}

fn setup_menu(app_handle: &AppHandle, items: Vec<(String, String)>) -> Result<()> {
    let menu = Menu::new(app_handle)?;
    for item in items {
        let menu_item = MenuItem::with_id(app_handle, item.0, item.1, true, None::<&str>)?;
        menu.append(&menu_item)?;
    }
    let tray = app_handle.tray_by_id("main").unwrap();
    tray.set_menu(Some(menu))?;
    Ok(())
}
