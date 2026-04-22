//! Web admin panel for mc-rs.
//!
//! Exposé en `pub fn serve(handle, cfg)` qui rend une `JoinHandle` tokio. Le
//! serveur principal (crate `mc-rs-server`) instancie un [`WebUiHandle`] à
//! partir de ses propres channels (console_tx, snapshot, event broadcast,
//! log broadcast) et spawn ce future.
//!
//! Les dépendances sont unidirectionnelles : `mc-rs-server` -> `mc-rs-webui`.
//! Le webui ne connaît rien du serveur à part les types exposés dans ce crate.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod handle;
pub mod metrics;
pub mod snapshot;

pub use config::WebUiConfig;
pub use error::{Error, Result};
pub use events::{LogLine, WebEvent};
pub use handle::WebUiHandle;
pub use snapshot::{PlayerSnapshot, ServerSnapshot};

mod routes;
mod state;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Lance le serveur web. Retourne le `JoinHandle` de la tâche tokio ; drop
/// sans await = arrêt propre via Drop du TcpListener (axum::serve gère).
pub fn serve(handle: WebUiHandle, config: WebUiConfig) -> JoinHandle<Result<()>> {
    tokio::spawn(async move { run(handle, config).await })
}

async fn run(handle: WebUiHandle, config: WebUiConfig) -> Result<()> {
    let bind_addr: SocketAddr = config
        .bind
        .parse()
        .map_err(|e| Error::BadConfig(format!("invalid bind '{}': {e}", config.bind)))?;

    if !bind_addr.ip().is_loopback() && !config.tls.enabled {
        warn!(
            "[webui] bind {} n'est pas loopback ET TLS désactivé — panel exposé en clair sur le réseau",
            bind_addr
        );
    }

    let state = state::AppState::new(handle, config.clone()).await?;
    let app = routes::router(Arc::new(state));

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(Error::Bind)?;

    let scheme = if config.tls.enabled { "https" } else { "http" };
    info!("[webui] listening on {scheme}://{}", bind_addr);

    // TLS path si feature `tls` + config.tls.enabled. Sinon HTTP normal.
    #[cfg(feature = "tls")]
    if config.tls.enabled {
        drop(listener); // axum-server gère son propre TcpListener
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
            &config.tls.cert_path,
            &config.tls.key_path,
        )
        .await
        .map_err(|e| Error::BadConfig(format!("TLS cert/key load: {e}")))?;
        axum_server::bind_rustls(bind_addr, tls_config)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| Error::Runtime(e.to_string()))?;
        return Ok(());
    }

    #[cfg(not(feature = "tls"))]
    if config.tls.enabled {
        return Err(Error::BadConfig(
            "TLS demandé mais crate compilé sans feature `tls`. Recompiler avec `--features tls`."
                .to_string(),
        ));
    }

    // ConnectInfo requis pour le rate limiter (IP par tentative sur /login).
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| Error::Runtime(e.to_string()))?;

    Ok(())
}
