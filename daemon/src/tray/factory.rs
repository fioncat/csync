use csync_misc::config::PathSet;

use crate::sync::send::SyncSender;

use super::api::ApiHandler;
use super::config::TrayConfig;

pub struct TrayFactory {
    cfg: TrayConfig,
}

impl TrayFactory {
    pub fn new(cfg: TrayConfig) -> Self {
        Self { cfg }
    }

    pub fn build_tray_api_handler(self, ps: PathSet, sync_tx: SyncSender) -> ApiHandler {
        let mut api = ApiHandler::new(ps, sync_tx);
        if self.cfg.text.enable {
            api.with_text(self.cfg.text.limit, self.cfg.text.default_action);
        }
        if self.cfg.image.enable {
            api.with_image(self.cfg.image.limit, self.cfg.image.default_action);
        }
        if self.cfg.file.enable {
            api.with_file(self.cfg.file.limit, self.cfg.file.default_action);
        }
        api.set_truncate_size(self.cfg.truncate_text);
        api
    }
}
