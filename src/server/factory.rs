use std::sync::Arc;

use anyhow::{Context, Result};
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslMethod};

use crate::secret::factory::SecretFactory;
use crate::server::authn::factory::AuthnFactory;
use crate::server::authn::token::factory::TokenFactory;
use crate::server::authz::factory::AuthzFactory;
use crate::server::db::factory::DbFactory;
use crate::server::handlers::api::ApiHandler;
use crate::server::handlers::healthz::HealthzHandler;
use crate::server::handlers::login::LoginHandler;

use super::config::ServerConfig;
use super::db::Database;
use super::recycle::factory::RecyclerFactory;
use super::recycle::Recycler;
use super::restful::{RestfulContext, RestfulServer};

pub struct ServerFactory {
    db: Arc<Database>,
    cfg: ServerConfig,
}

impl ServerFactory {
    pub fn new(cfg: ServerConfig) -> Result<Self> {
        let db_factory = DbFactory::new();
        let db = db_factory.build_db(&cfg.db).context("init database")?;
        Ok(Self { cfg, db })
    }

    pub fn build_server(&self) -> Result<RestfulServer> {
        let ssl = self.build_ssl()?;
        let ctx = self.build_context()?;

        let mut srv =
            RestfulServer::new(self.cfg.bind.clone(), ssl, ctx, self.cfg.payload_limit_mib);
        if self.cfg.keep_alive_secs > 0 {
            srv.set_keep_alive_secs(self.cfg.keep_alive_secs);
        }
        if self.cfg.workers > 0 {
            srv.set_workers(self.cfg.workers);
        }

        Ok(srv)
    }

    pub fn build_ssl(&self) -> Result<Option<SslAcceptorBuilder>> {
        if !self.cfg.ssl {
            return Ok(None);
        }

        let mut builder =
            SslAcceptor::mozilla_intermediate(SslMethod::tls()).context("init ssl acceptor")?;

        builder
            .set_private_key_file(&self.cfg.key_path, openssl::ssl::SslFiletype::PEM)
            .context("load ssl key file")?;
        builder
            .set_certificate_chain_file(&self.cfg.cert_path)
            .context("load ssl cert file")?;

        Ok(Some(builder))
    }

    pub fn build_context(&self) -> Result<Arc<RestfulContext>> {
        let token_factory = TokenFactory::new(&self.cfg.authn.token).context("init token")?;

        let authn_factory = AuthnFactory::new();
        let authn = authn_factory
            .build_authenticator(&self.cfg.authn, &token_factory)
            .context("init authenticator")?;

        let authz_factory = AuthzFactory::new();
        let authz = authz_factory.build_authorizer(&self.cfg.authz, self.db.clone());

        let secret_factory = SecretFactory::new();
        let secret = secret_factory
            .build_secret(&self.cfg.secret)
            .context("init secret")?;

        let api_handler = ApiHandler::new(authn, authz, self.db.clone(), Arc::new(secret));
        let healthz_handler = HealthzHandler::new();

        let token_generator = token_factory
            .build_token_generator()
            .context("init token generator")?;
        let admin_password = if !self.cfg.authn.admin_password.is_empty() {
            Some(self.cfg.authn.admin_password.clone())
        } else {
            None
        };
        let admin_allow_list = self.cfg.authn.admin_allow_list.clone();
        let login_handler = LoginHandler::new(
            admin_password,
            admin_allow_list,
            token_generator,
            self.db.clone(),
        );

        let ctx = RestfulContext {
            api_handler,
            healthz_handler,
            login_handler,
        };
        Ok(Arc::new(ctx))
    }

    pub fn build_recycler(&self) -> Result<Option<Recycler>> {
        let factory = RecyclerFactory::new();
        factory.build_recycler(&self.cfg.recycle, self.db.clone())
    }
}
