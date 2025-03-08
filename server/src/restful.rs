use std::sync::Arc;
use std::time::Duration;

use actix_web::web::{self, Data, PayloadConfig};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use csync_misc::api;
use csync_misc::api::Response;
use log::{info, warn};
use openssl::ssl::SslAcceptorBuilder;
use sd_notify::NotifyState;

use crate::context::ServerContext;
use crate::handlers::{self, convert_response};

pub struct RestfulServer {
    bind: String,

    ctx: Arc<ServerContext>,

    ssl: Option<SslAcceptorBuilder>,

    keep_alive_secs: Option<u64>,
    workers: Option<u64>,

    payload_limit_mib: Option<u64>,
}

impl RestfulServer {
    const DEFAULT_PAYLOAD_LIMIT_MIB: u64 = 10;

    pub fn new(bind: String, ctx: Arc<ServerContext>) -> Self {
        Self {
            ssl: None,
            ctx,
            keep_alive_secs: None,
            workers: None,
            bind,
            payload_limit_mib: None,
        }
    }

    pub fn set_ssl(&mut self, ssl: SslAcceptorBuilder) {
        self.ssl = Some(ssl);
    }

    pub fn set_keep_alive_secs(&mut self, keep_alive_secs: u64) {
        self.keep_alive_secs = Some(keep_alive_secs);
    }

    pub fn set_workers(&mut self, workers: u64) {
        self.workers = Some(workers);
    }

    pub fn set_payload_limit_mib(&mut self, payload_limit_mib: u64) {
        self.payload_limit_mib = Some(payload_limit_mib);
    }

    pub async fn run(mut self) -> Result<()> {
        let ctx = self.ctx.clone();
        let payload_limit = self
            .payload_limit_mib
            .unwrap_or(Self::DEFAULT_PAYLOAD_LIMIT_MIB)
            * 1024
            * 1024;

        let mut srv = HttpServer::new(move || {
            App::new()
                .app_data(Data::new(ctx.clone()))
                .app_data(PayloadConfig::new(payload_limit as usize))
                .service(
                    web::resource(api::user::USER_PATH)
                        .route(web::put().to(handlers::user::put_user_handler))
                        .route(web::get().to(handlers::user::get_user_handler))
                        .route(web::patch().to(handlers::user::patch_user_handler))
                        .route(web::delete().to(handlers::user::delete_user_handler)),
                )
                .service(
                    web::resource(api::blob::BLOB_PATH)
                        .route(web::put().to(handlers::blob::put_blob_handler))
                        .route(web::get().to(handlers::blob::get_blob_handler))
                        .route(web::patch().to(handlers::blob::patch_blob_handler))
                        .route(web::delete().to(handlers::blob::delete_blob_handler)),
                )
                .service(
                    web::resource(api::metadata::METADATA_PATH)
                        .route(web::get().to(handlers::metadata::get_metadata_handler)),
                )
                .service(
                    web::resource(api::user::GET_TOKEN_PATH)
                        .route(web::get().to(handlers::token::get_token_handler)),
                )
                .service(
                    web::resource(api::HEALTHZ_PATH)
                        .route(web::get().to(handlers::healthz::get_healthz_handler)),
                )
                .default_service(web::route().to(Self::default_handler))
        });

        if let Some(ssl) = self.ssl.take() {
            info!("Binding to https://{}", self.bind);
            srv = srv.bind_openssl(&self.bind, ssl).context("bind with ssl")?
        } else {
            warn!("Using HTTP (without SSL). THIS IS DANGEROUS, DO NOT USE IN PRODUCTION");
            info!("Binding to http://{}", self.bind);
            srv = srv.bind(&self.bind).context("bind without ssl")?
        };

        if let Some(keep_alive) = self.keep_alive_secs {
            srv = srv.keep_alive(Duration::from_secs(keep_alive));
        }
        if let Some(workers) = self.workers {
            srv = srv.workers(workers as usize);
        }

        sd_notify::notify(true, &[NotifyState::Ready]).context("notify systemd")?;
        info!("Starting restful server");
        srv.run().await.context("run server")?;

        info!("Server stopped by user");
        Ok(())
    }

    async fn default_handler(req: HttpRequest) -> HttpResponse {
        let path = req.uri().path().to_string();
        let method = req.method().as_str().to_string();
        let message = format!("No route to {method} {path}");
        let resp = Response::<()>::not_found(message);
        convert_response(resp)
    }
}
