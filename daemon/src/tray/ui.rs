use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::{Datelike, Local};
use log::{error, info};
use tauri::menu::{
    AboutMetadataBuilder, CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio::sync::mpsc;

use super::api::{ApiHandler, MenuData};

#[allow(deprecated)]
pub fn run_tray_ui(
    api: ApiHandler,
    default_menu: MenuData,
    notify_rx: mpsc::Receiver<()>,
) -> Result<()> {
    let api = Arc::new(api);

    let refresh_api = api.clone();

    info!("Starting system tray event loop");
    tauri::Builder::default()
        .setup(move |app| {
            // Hide the app icon from the dock(macOS) while keeping it in the menu bar
            // See: <https://github.com/tauri-apps/tauri/discussions/6038>
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            setup_menu(app.handle().clone(), default_menu, refresh_api.clone())?;
            let api = refresh_api.clone();
            let app = app.handle().clone();
            tokio::spawn(async move {
                auto_refresh_menu(app, api, notify_rx).await;
            });
            Ok(())
        })
        .on_menu_event(move |app, event| {
            let app = app.clone();
            let api = api.clone();

            match event.id.as_ref() {
                "quit" => {
                    info!("Quit application");
                    app.exit(0);
                    return;
                }
                "restart" => {
                    app.restart();
                }
                "auto_refresh" => {
                    api.update_auto_refresh();
                    info!("Auto refresh set to {}", api.get_auto_refresh());
                    return;
                }
                "refresh" => {
                    tokio::spawn(async move {
                        handle_result(refresh_menu(app, api).await);
                    });
                    return;
                }
                "settings" => {
                    tokio::spawn(async move {
                        handle_result(show_settings(app, api).await);
                    });
                    return;
                }
                _ => {}
            }

            tokio::spawn(async move {
                handle_result(handle_select(app, event.id.as_ref(), api).await);
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

async fn auto_refresh_menu(
    app: AppHandle,
    api: Arc<ApiHandler>,
    mut notify_rx: mpsc::Receiver<()>,
) {
    loop {
        notify_rx.recv().await.unwrap();

        if !api.get_auto_refresh() {
            continue;
        }

        info!("Update notification received, refreshing menu");
        if let Err(err) = refresh_menu(app.clone(), api.clone()).await {
            error!("Auo refresh menu error: {err:#}");
            continue;
        }
    }
}

async fn refresh_menu(app: AppHandle, api: Arc<ApiHandler>) -> Result<()> {
    let data = api.build_menu().await?;
    setup_menu(app, data, api)?;
    Ok(())
}

fn setup_menu(app: AppHandle, data: MenuData, api: Arc<ApiHandler>) -> Result<()> {
    let sep = PredefinedMenuItem::separator(&app)?;
    let menu = Menu::new(&app)?;

    if !data.texts.is_empty() {
        let text_sep = MenuItem::new(&app, "Texts", false, None::<&str>)?;
        menu.append(&text_sep)?;
    }

    for text in data.texts {
        let key = format!("text_{}", text.id);
        let value = text.text;

        let submenu = build_resource_submenu(&app, key, value, "Text")?;
        menu.append(&submenu)?;
    }

    menu.append(&sep)?;

    if !data.images.is_empty() {
        let image_sep = MenuItem::new(&app, "Images", false, None::<&str>)?;
        menu.append(&image_sep)?;
    }

    for image in data.images {
        let key = format!("image_{}", image.id);
        let value = format!("<{}>", image.size);

        let submenu = build_resource_submenu(&app, key, value, "Image")?;
        menu.append(&submenu)?;
    }

    menu.append(&sep)?;

    if !data.files.is_empty() {
        let file_sep = MenuItem::new(&app, "Files", false, None::<&str>)?;
        menu.append(&file_sep)?;
    }

    for file in data.files {
        let key = format!("file_{}", file.id);
        let value = format!("<{}, {}>", file.name, file.size);

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

    let refresh = MenuItem::with_id(&app, "refresh", "Refresh", true, Some("CmdOrCtrl+R"))?;
    menu.append(&refresh)?;

    let auto_refresh = CheckMenuItem::with_id(
        &app,
        "auto_refresh",
        "Auto Refresh",
        true,
        api.get_auto_refresh(),
        None::<&str>,
    )?;
    menu.append(&auto_refresh)?;

    menu.append(&sep)?;

    let settings = MenuItem::with_id(&app, "settings", "Settings", true, None::<&str>)?;
    menu.append(&settings)?;

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

    let restart_item = MenuItem::with_id(&app, "restart", "Restart", true, None::<&str>)?;
    menu.append(&restart_item)?;

    let quit_item = MenuItem::with_id(&app, "quit", "Quit", true, Some("CmdOrCtrl+Q"))?;
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

    if id.starts_with("upload_") {
        let kind = id.strip_prefix("upload_").unwrap();
        let path = match kind {
            // FIXME: Currently, Tauri's dialog doesn't support customizing the popup
            // position. This poses a problem for pure system tray applications like ours.
            // Specifically, on macOS, the dialog pops up above the menu bar, which isn't
            // user-friendly. We currently have no effective solution for this issue.
            // We need to wait for official support, see:
            //     <https://github.com/tauri-apps/plugins-workspace/issues/1306>
            "text" => app
                .dialog()
                .file()
                .set_title("Upload Text")
                .blocking_pick_file(),
            "image" => app
                .dialog()
                .file()
                .set_title("Upload Image")
                .add_filter("Image", &["png", "jpg", "jpeg"])
                .blocking_pick_file(),
            "file" => app
                .dialog()
                .file()
                .set_title("Upload File")
                .blocking_pick_file(),
            _ => unreachable!(),
        };

        if let Some(path) = path {
            let path = PathBuf::from(path.to_string());
            return match kind {
                "text" => api.upload_text(&path).await,
                "image" => api.upload_image(&path).await,
                "file" => api.upload_file(&path).await,
                _ => unreachable!(),
            };
        }

        return Ok(());
    }

    let fields = id.split('_').collect::<Vec<_>>();
    if fields.len() != 3 {
        bail!("invalid menu item id: {id}");
    }

    let kind = fields[0];
    let id = fields[1].parse::<u64>()?;
    let action = fields[2];

    match action {
        "copy" => match kind {
            "text" => api.copy_text(id).await?,
            "image" => api.copy_image(id).await?,
            "file" => api.copy_file(id).await?,
            _ => unreachable!(),
        },
        "open" => {
            let path = match kind {
                "text" => {
                    let path = api.get_tmp_path("text.txt");
                    api.save_text(id, &path).await?;
                    path
                }
                "image" => {
                    let path = api.get_tmp_path("image.png");
                    api.save_image(id, &path).await?;
                    path
                }
                "file" => {
                    let path = api.get_tmp_path("");
                    api.save_file(id, &path).await?
                }
                _ => unreachable!(),
            };

            let opener = app.opener();
            let path = format!("{}", path.display());
            opener.open_path(&path, None::<&str>)?;
        }
        "save" => {
            let path = match kind {
                "text" => app
                    .dialog()
                    .file()
                    .set_title("Save Text")
                    .blocking_save_file(),
                "image" => app
                    .dialog()
                    .file()
                    .set_title("Save Image")
                    .add_filter("Image", &["png", "jpg", "jpeg"])
                    .blocking_save_file(),
                "file" => app
                    .dialog()
                    .file()
                    .set_title("Save File")
                    .blocking_pick_folder(),
                _ => unreachable!(),
            };

            if let Some(path) = path {
                let path = PathBuf::from(path.to_string());
                match kind {
                    "text" => api.save_text(id, &path).await?,
                    "image" => api.save_image(id, &path).await?,
                    "file" => {
                        api.save_file(id, &path).await?;
                    }
                    _ => unreachable!(),
                }
            }
        }
        "delete" => match kind {
            "text" => api.delete_text(id).await?,
            "image" => api.delete_image(id).await?,
            "file" => api.delete_file(id).await?,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }

    refresh_menu(app, api).await?;
    Ok(())
}

async fn show_settings(app: AppHandle, _api: Arc<ApiHandler>) -> Result<()> {
    let window = match app.get_webview_window("settings") {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            &app,
            "settings",
            WebviewUrl::App(PathBuf::from("settings.html")),
        )
        .title("Csync Settings")
        .auto_resize()
        .inner_size(500.0, 500.0)
        // .resizable(false)
        .build()?,
    };
    window.show()?;
    Ok(())
}

fn handle_result(result: Result<()>) {
    if let Err(err) = result {
        error!("Tray Error: {err:#}");
    }
}
