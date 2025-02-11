use csync_misc::client::config::ClientConfig;
use csync_misc::config::PathSet;

use crate::sync::send::SyncSender;

use super::api::ApiHandler;
use super::config::TrayConfig;

pub struct TrayFactory {
    tray_cfg: TrayConfig,
    client_cfg: ClientConfig,
}

impl TrayFactory {
    pub fn new(tray_cfg: TrayConfig, client_cfg: ClientConfig) -> Self {
        Self {
            tray_cfg,
            client_cfg,
        }
    }

    pub fn build_tray_api_handler(self, ps: PathSet, sync_tx: SyncSender) -> ApiHandler {
        let mut api = ApiHandler::new(ps, sync_tx);
        if self.tray_cfg.text.enable {
            api.with_text(self.tray_cfg.text.limit);
        }
        if self.tray_cfg.image.enable {
            api.with_image(self.tray_cfg.image.limit);
        }
        if self.tray_cfg.file.enable {
            api.with_file(self.tray_cfg.file.limit);
        }
        api.set_truncate_size(self.tray_cfg.truncate_text);
        api
    }
}
