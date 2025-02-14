use std::sync::Arc;
use std::time::Duration;

use actix_web::http::StatusCode;
use actix_web::web::{self, Bytes, Data, PayloadConfig};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use csync_misc::types::response::CommonResponse;
use log::{info, warn};
use openssl::ssl::SslAcceptorBuilder;
use sd_notify::NotifyState;

use super::handlers::api::ApiHandler;
use super::handlers::healthz::HealthzHandler;
use super::handlers::login::LoginHandler;
use super::handlers::Handler;
use super::response::Response;

pub struct RestfulServer {
    ssl: Option<SslAcceptorBuilder>,
    ctx: Arc<RestfulContext>,

    keep_alive_secs: Option<u64>,
    workers: Option<u64>,

    bind: String,

    payload_limit_mib: usize,
}

pub struct RestfulContext {
    pub api_handler: ApiHandler,
    pub healthz_handler: HealthzHandler,
    pub login_handler: LoginHandler,
}

impl RestfulServer {
    const API_PATH: &str = "/api";
    const HEALTHZ_PATH: &str = "/healthz";
    const LOGIN_PATH: &str = "/login";

    pub fn new(
        bind: String,
        ssl: Option<SslAcceptorBuilder>,
        ctx: Arc<RestfulContext>,
        payload_limit_mib: usize,
    ) -> Self {
        Self {
            ssl,
            ctx,
            keep_alive_secs: None,
            workers: None,
            bind,
            payload_limit_mib,
        }
    }

    pub fn set_keep_alive_secs(&mut self, keep_alive_secs: u64) {
        self.keep_alive_secs = Some(keep_alive_secs);
    }

    pub fn set_workers(&mut self, workers: u64) {
        self.workers = Some(workers);
    }

    pub async fn run(mut self) -> Result<()> {
        let ctx = self.ctx.clone();
        let mut srv = HttpServer::new(move || {
            App::new()
                .app_data(Data::new(ctx.clone()))
                .app_data(PayloadConfig::new(self.payload_limit_mib * 1024 * 1024))
                .service(
                    web::scope(Self::API_PATH)
                        .route("/{path:.*}", web::get().to(Self::handle_api))
                        .route("/{path:.*}", web::put().to(Self::handle_api))
                        .route("/{path:.*}", web::patch().to(Self::handle_api))
                        .route("/{path:.*}", web::delete().to(Self::handle_api)),
                )
                .service(
                    web::resource(Self::HEALTHZ_PATH).route(web::get().to(Self::handle_healthz)),
                )
                .service(
                    web::scope(Self::LOGIN_PATH)
                        .route("/{path:.*}", web::post().to(Self::handle_login)),
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

    async fn handle_api(
        req: HttpRequest,
        body: Option<Bytes>,
        ctx: Data<Arc<RestfulContext>>,
    ) -> HttpResponse {
        let path = match Self::parse_path(Self::API_PATH, &req) {
            Some(path) => path,
            None => return Response::bad_request("Resource type is required").into(),
        };
        let body = Self::parse_body(body);

        ctx.api_handler.handle(&path, req, body).into()
    }

    async fn handle_healthz(
        req: HttpRequest,
        body: Option<Bytes>,
        ctx: Data<Arc<RestfulContext>>,
    ) -> HttpResponse {
        let body = Self::parse_body(body);

        ctx.healthz_handler.handle("", req, body).into()
    }

    async fn handle_login(
        req: HttpRequest,
        body: Option<Bytes>,
        ctx: Data<Arc<RestfulContext>>,
    ) -> HttpResponse {
        let path = match Self::parse_path(Self::LOGIN_PATH, &req) {
            Some(path) => path,
            None => return Response::bad_request("User name is required").into(),
        };
        let body = Self::parse_body(body);

        ctx.login_handler.handle(&path, req, body).into()
    }

    async fn default_handler(req: HttpRequest) -> HttpResponse {
        let path = req.uri().path().to_string();
        let method = req.method().as_str().to_string();
        let message = format!("No route to {method} {path}");
        let ret = CommonResponse {
            code: StatusCode::NOT_FOUND.into(),
            message: Some(message),
        };
        HttpResponse::NotFound().json(ret)
    }

    fn parse_path(route: &str, req: &HttpRequest) -> Option<String> {
        let path = req.uri().path().to_string();
        let path = path.strip_prefix(route)?;
        let path = path.trim_matches('/');
        if path.is_empty() {
            return None;
        }

        Some(String::from(path))
    }

    fn parse_body(body: Option<Bytes>) -> Option<Vec<u8>> {
        body.map(|b| b.to_vec())
    }
}
