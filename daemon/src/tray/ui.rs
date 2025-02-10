#![allow(deprecated)]

use anyhow::{bail, Context, Result};
use chrono::{Datelike, Local};
use log::{error, info};
use tauri::menu::{AboutMetadataBuilder, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;

use super::daemon::{MenuData, WriteRequest};

pub async fn build_and_run_tray_ui(
    default_menu: MenuData,
    menu_rx: mpsc::Receiver<MenuData>,
    write_tx: mpsc::Sender<WriteRequest>,
) -> Result<()> {
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
        .on_menu_event(move |app, event| {
            let id = event.id.as_ref();
            if id == "quit" {
                info!("Received quit event");
                std::process::exit(0);
            }

            let id = id.to_string();
            let write_tx = write_tx.clone();
            let app_handle = app.clone();
            tokio::spawn(async move {
                handle_event(app_handle, id, write_tx).await;
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .context("run system tray event loop")
}

async fn watch_menu_updates(app_handle: AppHandle, mut menu_rx: mpsc::Receiver<MenuData>) {
    info!("Watching menu updates");
    loop {
        let data = menu_rx.recv().await.unwrap();
        info!("Received menu update event");

        if let Err(e) = setup_menu(&app_handle, data) {
            error!("Failed to update system tray menu: {e:#}");
        }
    }
}

fn setup_menu(app_handle: &AppHandle, data: MenuData) -> Result<()> {
    let sep = PredefinedMenuItem::separator(app_handle)?;

    let menu = Menu::new(app_handle)?;

    for text in data.texts {
        let key = format!("text_{}", text.id);
        let value = text.text;

        let menu_item = MenuItem::with_id(app_handle, key, value, true, None::<&str>)?;
        menu.append(&menu_item)?;
    }

    menu.append(&sep)?;

    for image in data.images {
        let key = format!("image_{}", image.id);
        let value = format!("Image: <{}>", image.size);

        let save_key = format!("{}_save", key);
        let copy_key = format!("{}_copy", key);

        let menu_item = Submenu::with_id(app_handle, key, value, true)?;

        let save_item = MenuItem::with_id(app_handle, save_key, "Save", true, None::<&str>)?;
        menu_item.append(&save_item)?;

        let copy_item = MenuItem::with_id(app_handle, copy_key, "Copy", true, None::<&str>)?;
        menu_item.append(&copy_item)?;

        menu.append(&menu_item)?;
    }

    menu.append(&sep)?;

    for file in data.files {
        let key = format!("file_{}", file.id);
        let value = format!("File: <{}, {}>", file.name, file.size);

        let save_key = format!("{}_save", key);
        let copy_key = format!("{}_copy", key);

        let menu_item = Submenu::with_id(app_handle, key, value, true)?;

        let save_item = MenuItem::with_id(app_handle, save_key, "Save", true, None::<&str>)?;
        menu_item.append(&save_item)?;

        let copy_item = MenuItem::with_id(app_handle, copy_key, "Copy", true, None::<&str>)?;
        menu_item.append(&copy_item)?;

        menu.append(&menu_item)?;
    }

    menu.append(&sep)?;

    let year = Local::now().year();
    let copyright = format!("Copyright (c) {year} {}", env!("CARGO_PKG_AUTHORS"));

    let about = PredefinedMenuItem::about(
        app_handle,
        Some("About"),
        Some(
            AboutMetadataBuilder::new()
                .name(Some("Csync Daemon"))
                .version(Some(env!("CSYNC_VERSION")))
                .copyright(Some(copyright))
                .icon(app_handle.default_window_icon().cloned())
                .build(),
        ),
    )?;
    menu.append(&about)?;

    let quit_item = MenuItem::with_id(app_handle, "quit", "Quit", true, None::<&str>)?;
    menu.append(&quit_item)?;

    let tray = app_handle.tray_by_id("main").unwrap();
    tray.set_menu(Some(menu))?;
    Ok(())
}

async fn handle_event(app_handle: AppHandle, id: String, write_tx: mpsc::Sender<WriteRequest>) {
    let req = if id.starts_with("text_") {
        let id = match id.strip_prefix("text_").unwrap().parse::<u64>() {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to parse text menu item ID: {e:#}");
                return;
            }
        };

        WriteRequest::Text(id)
    } else if id.starts_with("image_") || id.starts_with("file_") {
        let is_image = id.starts_with("image_");
        let (id, path) = match get_item_id_and_path(&app_handle, &id, is_image) {
            Ok((id, path)) => (id, path),
            Err(e) => {
                error!("Failed to parse image/file menu item ID: {e:#}");
                return;
            }
        };

        if is_image {
            WriteRequest::Image(id, path)
        } else {
            WriteRequest::File(id, path)
        }
    } else {
        error!("Unknown menu item ID: {id}");
        return;
    };

    info!("Sending menu item click event: {:?}", req);
    write_tx.send(req).await.unwrap();
}

fn get_item_id_and_path(
    app_handle: &AppHandle,
    id: &str,
    is_image: bool,
) -> Result<(u64, Option<String>)> {
    let fields = id.split('_').collect::<Vec<_>>();
    if fields.len() != 3 {
        bail!("invalid length");
    }

    let id = fields[1].parse::<u64>().context("parse id")?;

    match fields[2] {
        "save" => {
            let mut dialog = app_handle.dialog().file();
            if is_image {
                dialog = dialog.add_filter("Image", &["png", "jpg", "jpeg"]);
            }

            let path = dialog.blocking_save_file();
            match path {
                Some(path) => Ok((id, Some(path.to_string()))),
                None => bail!("no path selected"),
            }
        }
        "copy" => Ok((id, None)),
        _ => bail!("invalid action {}", fields[2]),
    }
}
