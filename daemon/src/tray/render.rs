use std::borrow::Cow;

use anyhow::Result;
use chrono::{Datelike, Utc};
use log::debug;
use tauri::image::Image;
use tauri::menu::{
    AboutMetadataBuilder, CheckMenuItem, IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Wry};

use super::state::TrayState;

pub struct Renderer {
    app: AppHandle,
    menu: Menu<Wry>,
    state: TrayState,
    sep: PredefinedMenuItem<Wry>,
}

impl Renderer {
    const ERROR_TITLE: &'static str = "Server Error, please check your network and logs";

    pub fn new(app: AppHandle, state: TrayState) -> Result<Self> {
        let menu = Menu::new(&app)?;
        let sep = PredefinedMenuItem::separator(&app)?;
        Ok(Self {
            app,
            menu,
            state,
            sep,
        })
    }

    pub fn render(self) -> Result<()> {
        debug!("Begin to render tray menu with state {:?}", self.state);
        self.render_title()?;
        self.render_items()?;

        self.render_sep()?;

        self.render_more()?;
        self.render_quit()?;

        let tray = self.app.tray_by_id("main").unwrap();
        tray.set_menu(Some(self.menu))?;

        debug!("Tray menu rendered");
        Ok(())
    }

    fn render_title(&self) -> Result<()> {
        let error_icon_data = include_bytes!("./../../icons/error.png");
        let ok_icon_data = include_bytes!("./../../icons/checkmark.png");

        let error_icon = Image::from_bytes(error_icon_data).unwrap();
        let ok_icon = Image::from_bytes(ok_icon_data).unwrap();

        let title_item = if self.state.fetch_error || self.state.rev_error {
            IconMenuItem::new(
                &self.app,
                Self::ERROR_TITLE,
                false,
                Some(error_icon.clone()),
                None::<&str>,
            )?
        } else {
            let word = if self.state.total > 1 {
                "items"
            } else {
                "item"
            };
            let title = format!("Server ready, with {} {word}", self.state.total);
            IconMenuItem::new(&self.app, title, false, Some(ok_icon), None::<&str>)?
        };

        self.menu.append(&title_item)?;

        Ok(())
    }

    fn render_items(&self) -> Result<()> {
        for item in self.state.items.iter() {
            let key = item.id.to_string();

            // FIXME: We are currently limited to using emojis as Tauri's Submenu does not
            // support icon settings. This should be updated to use icons when Submenu icon
            // support becomes available, which will provide better compatibility.
            // See: <https://github.com/tauri-apps/tauri/issues/11796>
            let summary = if item.pin {
                Cow::Owned(format!("⭐ {}", item.summary))
            } else {
                Cow::Borrowed(&item.summary)
            };

            let submenu = Submenu::with_id(&self.app, &key, summary.as_ref(), true)?;

            let copy_key = format!("copy_{key}");
            let copy_item = MenuItem::with_id(&self.app, &copy_key, "Copy", true, None::<&str>)?;

            let open_key = format!("open_{key}");
            let open_item = MenuItem::with_id(&self.app, &open_key, "Open", true, None::<&str>)?;

            let save_key = format!("save_{key}");
            let save_item = MenuItem::with_id(&self.app, &save_key, "Save", true, None::<&str>)?;

            let pin_key = if item.pin {
                format!("unpin_{key}")
            } else {
                format!("pin_{key}")
            };
            let pin_item =
                CheckMenuItem::with_id(&self.app, &pin_key, "Pin", true, item.pin, None::<&str>)?;

            let delete_key = format!("delete_{key}");
            let delete_item =
                MenuItem::with_id(&self.app, &delete_key, "Delete", true, None::<&str>)?;

            submenu.append_items(&[&copy_item, &open_item, &save_item, &pin_item, &delete_item])?;
            self.menu.append(&submenu)?;
        }

        Ok(())
    }

    fn render_more(&self) -> Result<()> {
        let more = Submenu::with_id(&self.app, "more", "More", true)?;

        let upload_item = Submenu::with_id(&self.app, "upload", "Upload", true)?;
        let upload_text =
            MenuItem::with_id(&self.app, "upload_text", "Upload Text", true, None::<&str>)?;
        let upload_image = MenuItem::with_id(
            &self.app,
            "upload_image",
            "Upload Image",
            true,
            None::<&str>,
        )?;
        let upload_file =
            MenuItem::with_id(&self.app, "upload_file", "Upload File", true, None::<&str>)?;
        upload_item.append_items(&[&upload_text, &upload_image, &upload_file])?;
        more.append(&upload_item)?;

        let config = Submenu::with_id(&self.app, "config", "Configuration", true)?;
        let client_config =
            MenuItem::with_id(&self.app, "client_config", "Client", true, None::<&str>)?;
        let daemon_config =
            MenuItem::with_id(&self.app, "daemon_config", "Daemon", true, None::<&str>)?;
        config.append_items(&[&client_config, &daemon_config])?;
        more.append(&config)?;

        let logs = MenuItem::with_id(&self.app, "logs", "Logs", true, None::<&str>)?;
        more.append(&logs)?;

        let year = Utc::now().year();
        let copyright = format!("Copyright (c) {year} {}", env!("CARGO_PKG_AUTHORS"));

        let about = PredefinedMenuItem::about(
            &self.app,
            Some("About"),
            Some(
                AboutMetadataBuilder::new()
                    .name(Some("Csync"))
                    .version(Some(env!("CSYNC_VERSION")))
                    .copyright(Some(copyright))
                    .icon(self.app.default_window_icon().cloned())
                    .build(),
            ),
        )?;
        more.append(&about)?;

        self.menu.append(&more)?;

        Ok(())
    }

    fn render_quit(&self) -> Result<()> {
        let quit_item = MenuItem::with_id(&self.app, "quit", "Quit", true, Some("CmdOrCtrl+Q"))?;
        self.menu.append(&quit_item)?;
        Ok(())
    }

    fn render_sep(&self) -> Result<()> {
        self.menu.append(&self.sep)?;
        Ok(())
    }
}
