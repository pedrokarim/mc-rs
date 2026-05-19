//! `/config` — read + POST écriture atomique de `server.toml`.
//!
//! Validation : parse TOML + présence des sections `[server]`, `[world]`,
//! `[gameplay]`, `[logging]`, `[webui]` comme tables. Écriture atomique via
//! fichier `.tmp` + rename (évite la corruption en cas de crash pendant l'écriture).

use askama::Template;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::db::Role;
use crate::state::AppState;

const CONFIG_PATH: &str = "server.toml";
const REQUIRED_SECTIONS: &[&str] = &["server", "world", "gameplay", "logging", "webui"];

#[derive(Template)]
#[template(path = "config.html")]
struct ConfigTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
    toml_content: String,
    path: String,
}

pub async fn get_config(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let path = PathBuf::from(CONFIG_PATH);
    let content =
        std::fs::read_to_string(&path).unwrap_or_else(|e| format!("(lecture impossible : {e})"));

    let tpl = ConfigTemplate {
        current_page: "config",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
        toml_content: content,
        path: path.display().to_string(),
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] config render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}

pub async fn post_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "admin only").into_response();
    }

    let Ok(text) = std::str::from_utf8(&body) else {
        return (StatusCode::BAD_REQUEST, "body must be UTF-8").into_response();
    };

    // Validation : parse TOML puis vérif sections.
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("TOML invalide : {e}")).into_response(),
    };
    if let toml::Value::Table(ref t) = parsed {
        for req in REQUIRED_SECTIONS {
            match t.get(*req) {
                Some(toml::Value::Table(_)) => {}
                Some(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Section [{req}] doit être une table"),
                    )
                        .into_response()
                }
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Section [{req}] manquante"),
                    )
                        .into_response()
                }
            }
        }
    } else {
        return (StatusCode::BAD_REQUEST, "racine doit être une table TOML").into_response();
    }

    // Écriture atomique : tempfile + rename.
    let path = Path::new(CONFIG_PATH);
    let tmp_path = path.with_extension("toml.tmp");
    if let Err(e) = atomic_write(&tmp_path, path, text.as_bytes()) {
        tracing::error!("[webui] config write: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Erreur écriture : {e}"),
        )
            .into_response();
    }

    // Audit + event broadcast.
    if let Some(db) = state.db.clone() {
        let _ = db
            .audit_log(
                Some(user.id),
                Some(&user.name),
                "config.edit",
                serde_json::json!({ "bytes": text.len() }),
            )
            .await;
    }
    let _ = state.handle.event_tx.send(crate::WebEvent::AdminAction {
        actor: user.name.clone(),
        action: "config.edit".to_string(),
        detail: format!(
            "{} bytes written — restart required for some fields",
            text.len()
        ),
    });

    tracing::info!(
        "[webui] config updated by {} ({} bytes)",
        user.name,
        text.len()
    );
    (
        StatusCode::OK,
        "Sauvegardé. Redémarrez le serveur pour appliquer les changements non-hot.",
    )
        .into_response()
}

fn atomic_write(tmp: &Path, target: &Path, data: &[u8]) -> std::io::Result<()> {
    {
        let mut f = std::fs::File::create(tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    std::fs::rename(tmp, target)?;
    Ok(())
}
