//! État partagé injecté dans chaque handler Axum via `State<Arc<AppState>>`.

use std::sync::Arc;

use crate::auth::{JwtCodec, RateLimiter};
use crate::config::WebUiConfig;
use crate::db::WebDb;
use crate::error::Result;
use crate::handle::WebUiHandle;

pub struct AppState {
    pub handle: WebUiHandle,
    pub config: WebUiConfig,
    pub db: Option<Arc<dyn WebDb>>,
    pub jwt: Option<JwtCodec>,
    pub login_ratelimit: RateLimiter,
}

impl AppState {
    /// Construit l'état. Initialise la DB + JWT si un backend est configuré.
    /// En cas d'erreur DB, on loggue et on continue avec `db=None` — le serveur
    /// web peut alors servir `/api/health` mais les routes auth seront HS.
    pub async fn new(handle: WebUiHandle, config: WebUiConfig) -> Result<Self> {
        let (db, jwt) = match crate::db::open_db(&config.database_url).await {
            Ok(db) => {
                db.migrate().await?;
                let secret = crate::auth::jwt::load_or_init_secret(db.as_ref()).await?;
                let ttl = (config.session_duration_hours as i64) * 3600;
                let codec = JwtCodec::new(&secret, ttl);
                (Some(db), Some(codec))
            }
            Err(e) => {
                tracing::error!("[webui] DB init failed: {e} — auth routes will return 500");
                (None, None)
            }
        };

        Ok(Self {
            handle,
            config,
            db,
            jwt,
            login_ratelimit: RateLimiter::new(),
        })
    }
}
