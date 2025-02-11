#![allow(deprecated)]

use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{Datelike, Local};
use log::{error, info};
use tauri::menu::{AboutMetadataBuilder, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconEvent;
use tauri::{AppHandle, Wry};

use super::api::{ApiHandler, MenuData};

pub fn run_tray_ui(api: ApiHandler, default_menu: MenuData) -> Result<()> {
    let api = Arc::new(api);

    let icon_api = api.clone();

    info!("Starting system tray event loop");
    tauri::Builder::default()
        .on_tray_icon_event(move |app, event| {
            println!("event!!");
            if let TrayIconEvent::Click { .. } = event {
                let app = app.clone();
                let api = icon_api.clone();

                tokio::spawn(async move {
                    handle_result(handle_icon_click(app, api).await);
                });
            }
        })
        .setup(move |app| {
            // Hide the app icon from the dock(macOS) while keeping it in the menu bar
            // See: <https://github.com/tauri-apps/tauri/discussions/6038>
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            setup_menu(app.handle().clone(), default_menu)?;
            Ok(())
        })
        .on_menu_event(move |app, event| {
            let app = app.clone();
            let api = api.clone();
            tokio::spawn(async move {
                handle_result(handle_select(app, event.id.as_ref(), api).await);
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .context("run system tray event loop")
}

async fn handle_icon_click(app: AppHandle, api: Arc<ApiHandler>) -> Result<()> {
    info!("Icon clicked, refreshing menu");
    let data = api.build_menu().await?;
    setup_menu(app, data)
}

fn setup_menu(app: AppHandle, data: MenuData) -> Result<()> {
    let sep = PredefinedMenuItem::separator(&app)?;
    let menu = Menu::new(&app)?;

    for text in data.texts {
        let key = format!("text_{}", text.id);
        let value = text.text;

        let submenu = build_resource_submenu(&app, key, value, "Text")?;
        menu.append(&submenu)?;
    }

    menu.append(&sep)?;

    for image in data.images {
        let key = format!("image_{}", image.id);
        let value = format!("<Image, {}>", image.size);

        let submenu = build_resource_submenu(&app, key, value, "Image")?;
        menu.append(&submenu)?;
    }

    menu.append(&sep)?;

    for file in data.files {
        let key = format!("file_{}", file.id);
        let value = format!("<File, {}, {}>", file.name, file.size);

        let submenu = build_resource_submenu(&app, key, value, "File")?;
        menu.append(&submenu)?;
    }

    menu.append(&sep)?;

    let upload_item = Submenu::with_id(&app, "upload", "Upload", true)?;
    let upload_text = MenuItem::with_id(&app, "upload_text", "Upload Text", true, None::<&str>)?;
    let upload_image = MenuItem::with_id(&app, "upload_image", "Upload Image", true, None::<&str>)?;
    let upload_file = MenuItem::with_id(&app, "upload_file", "Upload File", true, None::<&str>)?;
    upload_item.append_items(&[&upload_text, &upload_image, &upload_file])?;
    menu.append(&upload_item)?;

    let year = Local::now().year();
    let copyright = format!("Copyright (c) {year} {}", env!("CARGO_PKG_AUTHORS"));

    let about = PredefinedMenuItem::about(
        &app,
        Some("About"),
        Some(
            AboutMetadataBuilder::new()
                .name(Some("Csync Daemon"))
                .version(Some(env!("CSYNC_VERSION")))
                .copyright(Some(copyright))
                .icon(app.default_window_icon().cloned())
                .build(),
        ),
    )?;
    menu.append(&about)?;

    let quit_item = MenuItem::with_id(&app, "quit", "Quit", true, None::<&str>)?;
    menu.append(&quit_item)?;

    let tray = app.tray_by_id("main").unwrap();
    tray.set_menu(Some(menu))?;

    Ok(())
}

fn build_resource_submenu(
    app: &AppHandle,
    key: String,
    value: String,
    name: &str,
) -> Result<Submenu<Wry>> {
    let submenu = Submenu::with_id(app, &key, value, true)?;

    let copy_key = format!("{key}_copy");
    let copy_item = MenuItem::with_id(app, copy_key, format!("Copy {name}"), true, None::<&str>)?;

    let open_key = format!("{key}_open");
    let open_item = MenuItem::with_id(app, open_key, format!("Open {name}"), true, None::<&str>)?;

    let save_key = format!("{key}_save");
    let save_item = MenuItem::with_id(app, save_key, format!("Save {name}"), true, None::<&str>)?;

    let delete_key = format!("{key}_delete");
    let delete_item = MenuItem::with_id(
        app,
        delete_key,
        format!("Delete {name}"),
        true,
        None::<&str>,
    )?;

    submenu.append_items(&[&copy_item, &open_item, &save_item, &delete_item])?;
    Ok(submenu)
}

async fn handle_select(app: AppHandle, id: &str, api: Arc<ApiHandler>) -> Result<()> {
    info!("Selected menu item: {id}");
    Ok(())
}

fn handle_result(result: Result<()>) {
    if let Err(err) = result {
        error!("Tray Error: {err}");
    }
}
