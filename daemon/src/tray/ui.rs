use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, io};

use anyhow::{bail, Context, Result};
use chrono::{Datelike, Local};
use log::{error, info};
use tauri::image::Image;
use tauri::menu::{
    AboutMetadataBuilder, CheckMenuItem, IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, WindowEvent, Wry};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio::time::interval;

use super::api::{ApiHandler, MenuData};
use super::config::TrayAction;

#[allow(deprecated)]
pub fn run_tray_ui(api: ApiHandler, default_menu: MenuData) -> Result<()> {
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
                auto_refresh_menu(app, api).await;
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
                    info!("Restart application");
                    app.restart();
                }
                "auto_refresh" => {
                    api.update_auto_refresh();
                    info!("Auto refresh set to {}", api.get_auto_refresh());
                    return;
                }
                "refresh" => {
                    info!("Refresh menu");
                    tokio::spawn(async move {
                        handle_result(refresh_menu(app, api).await);
                    });
                    return;
                }
                "client_config" => {
                    info!("Open client configuration");
                    handle_result(open_config(app, api, "client"));
                    return;
                }
                "daemon_config" => {
                    info!("Open daemon configuration");
                    handle_result(open_config(app, api, "daemon"));
                    return;
                }
                "logs" => {
                    info!("Open logs file");
                    handle_result(open_logs(app, api));
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

async fn auto_refresh_menu(app: AppHandle, api: Arc<ApiHandler>) {
    let mut intv = interval(Duration::from_secs(1));
    let mut rev = None;
    loop {
        intv.tick().await;

        if !api.get_auto_refresh() {
            continue;
        }

        let cur_rev = match api.get_revision().await {
            Ok(rev) => rev,
            Err(e) => {
                error!("Get server revision error: {e:#}");
                continue;
            }
        };

        match rev {
            Some(rev) if rev == cur_rev => continue,
            None => {
                rev = Some(cur_rev);
                continue;
            }
            _ => {}
        }

        info!("Server revision updated to {cur_rev}, refreshing menu");
        if let Err(err) = refresh_menu(app.clone(), api.clone()).await {
            error!("Auo refresh menu error: {err:#}");
            continue;
        }
        rev = Some(cur_rev);
    }
}

async fn refresh_menu(app: AppHandle, api: Arc<ApiHandler>) -> Result<()> {
    let data = api.build_menu().await?;
    setup_menu(app, data, api)?;
    info!("Tray menu refreshed");
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

        append_resource_menu(&app, &menu, key, value, "Text", api.get_text_action())?;
    }

    menu.append(&sep)?;

    if !data.images.is_empty() {
        let image_sep = MenuItem::new(&app, "Images", false, None::<&str>)?;
        menu.append(&image_sep)?;
    }

    for image in data.images {
        let key = format!("image_{}", image.id);
        let value = format!("<{}>", image.size);

        append_resource_menu(&app, &menu, key, value, "Image", api.get_image_action())?;
    }

    menu.append(&sep)?;

    if !data.files.is_empty() {
        let file_sep = MenuItem::new(&app, "Files", false, None::<&str>)?;
        menu.append(&file_sep)?;
    }

    for file in data.files {
        let key = format!("file_{}", file.id);
        let value = format!("<{}, {}>", file.name, file.size);

        append_resource_menu(&app, &menu, key, value, "File", api.get_file_action())?;
    }

    menu.append(&sep)?;

    let more = Submenu::with_id(&app, "more", "More", true)?;

    let upload_item = Submenu::with_id(&app, "upload", "Upload", true)?;
    let upload_text = MenuItem::with_id(&app, "upload_text", "Upload Text", true, None::<&str>)?;
    let upload_image = MenuItem::with_id(&app, "upload_image", "Upload Image", true, None::<&str>)?;
    let upload_file = MenuItem::with_id(&app, "upload_file", "Upload File", true, None::<&str>)?;
    upload_item.append_items(&[&upload_text, &upload_image, &upload_file])?;
    more.append(&upload_item)?;

    let refresh = MenuItem::with_id(&app, "refresh", "Refresh", true, Some("CmdOrCtrl+R"))?;
    more.append(&refresh)?;

    let auto_refresh = CheckMenuItem::with_id(
        &app,
        "auto_refresh",
        "Auto Refresh",
        true,
        api.get_auto_refresh(),
        None::<&str>,
    )?;
    more.append(&auto_refresh)?;

    let action_item = Submenu::with_id(&app, "action", "Default Action", true)?;

    let text_action = build_resource_action_submenu(&app, "text", "Text", api.get_text_action())?;
    let image_action =
        build_resource_action_submenu(&app, "image", "Image", api.get_image_action())?;
    let file_action = build_resource_action_submenu(&app, "file", "File", api.get_file_action())?;
    action_item.append_items(&[&text_action, &image_action, &file_action])?;
    more.append(&action_item)?;

    let config = Submenu::with_id(&app, "config", "Configuration", true)?;
    let client_config = MenuItem::with_id(&app, "client_config", "Client", true, None::<&str>)?;
    let daemon_config = MenuItem::with_id(&app, "daemon_config", "Daemon", true, None::<&str>)?;
    config.append_items(&[&client_config, &daemon_config])?;
    more.append(&config)?;

    let logs = MenuItem::with_id(&app, "logs", "Logs", true, None::<&str>)?;
    more.append(&logs)?;

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
    more.append(&about)?;

    menu.append(&more)?;
    let quit_item = MenuItem::with_id(&app, "quit", "Quit", true, Some("CmdOrCtrl+Q"))?;
    menu.append(&quit_item)?;

    let tray = app.tray_by_id("main").unwrap();
    tray.set_menu(Some(menu))?;

    Ok(())
}

fn append_resource_menu(
    app: &AppHandle,
    menu: &Menu<Wry>,
    key: String,
    value: String,
    name: &str,
    action: TrayAction,
) -> Result<()> {
    match action {
        TrayAction::Copy => {
            let star_icon = Image::from_bytes(include_bytes!("../../icons/star.png"))?;
            let item = IconMenuItem::with_id(app, key, value, true, Some(star_icon), None::<&str>)?;
            // let key = format!("{key}_copy");
            // let item = MenuItem::with_id(app, key, value, true, None::<&str>)?;
            menu.append(&item)?;
        }
        TrayAction::Open => {
            let key = format!("{key}_open");
            let item = MenuItem::with_id(app, key, value, true, None::<&str>)?;
            menu.append(&item)?;
        }
        TrayAction::Save => {
            let key = format!("{key}_save");
            let item = MenuItem::with_id(app, key, value, true, None::<&str>)?;
            menu.append(&item)?;
        }
        TrayAction::Delete => {
            let key = format!("{key}_delete");
            let item = MenuItem::with_id(app, key, value, true, None::<&str>)?;
            menu.append(&item)?;
        }
        TrayAction::None => {
            let submenu = build_resource_submenu(app, key.clone(), value, name)?;
            menu.append(&submenu)?;
        }
    };
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

fn build_resource_action_submenu(
    app: &AppHandle,
    key: &str,
    name: &str,
    action: TrayAction,
) -> Result<Submenu<Wry>> {
    let submenu = Submenu::with_id(app, format!("action_{key}"), name, true)?;

    let copy_item = CheckMenuItem::with_id(
        app,
        format!("action_{key}_copy"),
        "Copy",
        true,
        matches!(action, TrayAction::Copy),
        None::<&str>,
    )?;
    let open_item = CheckMenuItem::with_id(
        app,
        format!("action_{key}_open"),
        "Open",
        true,
        matches!(action, TrayAction::Open),
        None::<&str>,
    )?;
    let save_item = CheckMenuItem::with_id(
        app,
        format!("action_{key}_save"),
        "Save",
        true,
        matches!(action, TrayAction::Save),
        None::<&str>,
    )?;
    let delete_item = CheckMenuItem::with_id(
        app,
        format!("action_{key}_delete"),
        "Delete",
        true,
        matches!(action, TrayAction::Delete),
        None::<&str>,
    )?;

    submenu.append_items(&[&copy_item, &open_item, &save_item, &delete_item])?;

    Ok(submenu)
}

async fn handle_select(app: AppHandle, id: &str, api: Arc<ApiHandler>) -> Result<()> {
    info!("Selected menu item: {id}");

    if id.starts_with("action_") {
        let id = id.strip_prefix("action_").unwrap();
        let fields = id.split('_').collect::<Vec<_>>();
        if fields.len() != 2 {
            bail!("invalid menu action id: {id}");
        }
        let kind = fields[0];
        let action = match fields[1] {
            "copy" => TrayAction::Copy,
            "open" => TrayAction::Open,
            "save" => TrayAction::Save,
            "delete" => TrayAction::Delete,
            _ => unreachable!(),
        };

        let current_action = match kind {
            "text" => api.get_text_action(),
            "image" => api.get_image_action(),
            "file" => api.get_file_action(),
            _ => unreachable!(),
        };

        let update_action = if current_action == action {
            TrayAction::None
        } else {
            action
        };

        match kind {
            "text" => api.set_text_action(update_action),
            "image" => api.set_image_action(update_action),
            "file" => api.set_file_action(update_action),
            _ => unreachable!(),
        }

        refresh_menu(app, api).await?;
        return Ok(());
    }

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
    Ok(())
}

fn open_config(app: AppHandle, api: Arc<ApiHandler>, kind: &str) -> Result<()> {
    let path = api.get_config_path(kind);

    match fs::metadata(&path) {
        Ok(meta) => {
            if meta.is_dir() {
                bail!("config path is a directory: {}", path.display());
            }
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let default_data = match kind {
                "client" => include_bytes!("../../../config/client.toml").to_vec(),
                "daemon" => include_bytes!("../../../config/daemon.toml").to_vec(),
                _ => unreachable!(),
            };
            fs::write(&path, default_data)?;
        }
        Err(e) => return Err(e).context("read config file"),
    }

    let opener = app.opener();
    let path = format!("{}", path.display());
    opener.open_path(&path, None::<&str>)?;

    Ok(())
}

fn open_logs(app: AppHandle, api: Arc<ApiHandler>) -> Result<()> {
    let path = api.get_logs_path();
    let opener = app.opener();
    let path = format!("{}", path.display());
    opener.open_path(&path, None::<&str>)?;
    Ok(())
}

fn handle_result(result: Result<()>) {
    if let Err(err) = result {
        error!("Tray Error: {err:#}");
    }
}
