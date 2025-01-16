use std::sync::Arc;

use actix_web::web::{self, Bytes, Data};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use log::info;
use sd_notify::NotifyState;
use tokio::sync::mpsc;

use crate::filelock::GlobalLock;
use crate::humanize::human_bytes;
use crate::imghdr::is_data_image;

pub struct DaemonServer {
    ctx: Arc<DaemonContext>,

    port: u16,

    lock: Option<GlobalLock>,
}

pub struct DaemonContext {
    pub text_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub image_tx: Option<mpsc::Sender<Vec<u8>>>,
}

impl DaemonServer {
    pub fn new(ctx: Arc<DaemonContext>, port: u16) -> Self {
        Self {
            ctx,
            port,
            lock: None,
        }
    }

    pub fn set_global_lock(&mut self, lock: GlobalLock) {
        self.lock = Some(lock);
    }

    pub async fn run(self) -> Result<()> {
        let ctx = self.ctx.clone();
        let bind = format!("127.0.0.1:{}", self.port);
        info!("Binding to: http://{bind}");

        let srv = HttpServer::new(move || {
            App::new()
                .app_data(Data::new(ctx.clone()))
                .default_service(web::route().to(Self::handle))
        })
        .bind(&bind)
        .context("bind daemon server")?;

        sd_notify::notify(true, &[NotifyState::Ready]).context("notify systemd")?;
        info!("Starting daemon server");
        srv.run().await.context("run daemon server")?;

        info!("Daemon server stopped by user");
        Ok(())
    }

    async fn handle(
        req: HttpRequest,
        body: Option<Bytes>,
        ctx: Data<Arc<DaemonContext>>,
    ) -> HttpResponse {
        let method = req.method().as_str().to_lowercase();
        if method != "put" {
            return HttpResponse::MethodNotAllowed().finish();
        }

        let data = match body {
            Some(data) => data.to_vec(),
            None => return HttpResponse::BadRequest().finish(),
        };
        if is_data_image(&data) {
            info!(
                "Received {} image data from user",
                human_bytes(data.len() as u64)
            );
            ctx.send_image(data).await;
        } else {
            if String::from_utf8(data.clone()).is_err() {
                return HttpResponse::BadRequest().finish();
            }
            info!(
                "Received {} text data from user",
                human_bytes(data.len() as u64)
            );
            ctx.send_text(data).await;
        }

        HttpResponse::Ok().finish()
    }
}

impl DaemonContext {
    async fn send_text(&self, text: Vec<u8>) {
        if let Some(ref tx) = self.text_tx {
            tx.send(text).await.unwrap();
        }
    }

    async fn send_image(&self, image: Vec<u8>) {
        if let Some(ref tx) = self.image_tx {
            tx.send(image).await.unwrap();
        }
    }
}
