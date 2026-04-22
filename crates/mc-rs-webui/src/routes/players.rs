//! `/players` — liste joueurs + actions (kick/op/deop/gamemode).
//! Toutes les actions fire une commande console (pas de code métier duppliqué).

use askama::Template;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Template)]
#[template(path = "players.html")]
struct PlayersTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
}

pub async fn get_players(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let tpl = PlayersTemplate {
        current_page: "players",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] players render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}

async fn fire_command(
    state: &AppState,
    user_id: uuid::Uuid,
    user_name: &str,
    action: &str,
    command: String,
) -> Response {
    if state.handle.console_tx.send(command.clone()).is_err() {
        return (StatusCode::SERVICE_UNAVAILABLE, "console closed").into_response();
    }
    if let Some(db) = state.db.clone() {
        let _ = db
            .audit_log(
                Some(user_id),
                Some(user_name),
                action,
                serde_json::json!({ "command": command }),
            )
            .await;
    }
    let _ = state.handle.event_tx.send(crate::WebEvent::AdminAction {
        actor: user_name.to_string(),
        action: action.to_string(),
        detail: command,
    });
    (StatusCode::OK, "queued").into_response()
}

fn sanitize_name(name: &str) -> Option<String> {
    // Only ASCII alphanumeric + underscore for safety (pas d'injection via espaces/quotes).
    if name.is_empty() || name.len() > 32 {
        return None;
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(name.to_string())
}

pub async fn post_kick(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(safe) = sanitize_name(&name) else {
        return (StatusCode::BAD_REQUEST, "invalid player name").into_response();
    };
    fire_command(&state, user.id, &user.name, "player.kick", format!("/kick {safe}")).await
}

pub async fn post_op(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(safe) = sanitize_name(&name) else {
        return (StatusCode::BAD_REQUEST, "invalid player name").into_response();
    };
    fire_command(&state, user.id, &user.name, "player.op", format!("/op {safe}")).await
}

pub async fn post_deop(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(safe) = sanitize_name(&name) else {
        return (StatusCode::BAD_REQUEST, "invalid player name").into_response();
    };
    fire_command(&state, user.id, &user.name, "player.deop", format!("/deop {safe}")).await
}

#[derive(Deserialize)]
pub struct GamemodeForm {
    mode: String,
}

pub async fn post_gamemode(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Form(form): Form<GamemodeForm>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(safe) = sanitize_name(&name) else {
        return (StatusCode::BAD_REQUEST, "invalid player name").into_response();
    };
    let mode_idx: i32 = match form.mode.parse() {
        Ok(n) if (0..=3).contains(&n) => n,
        _ => return (StatusCode::BAD_REQUEST, "invalid gamemode").into_response(),
    };
    let mode_name = ["survival", "creative", "adventure", "spectator"][mode_idx as usize];
    fire_command(
        &state,
        user.id,
        &user.name,
        "player.gamemode",
        format!("/gamemode {mode_name} {safe}"),
    )
    .await
}
