//! `/world` — panneau time/weather/difficulty. Tout via commandes console.

use askama::Template;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Template)]
#[template(path = "world.html")]
struct WorldTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
}

pub async fn get_world(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let tpl = WorldTemplate {
        current_page: "config",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] world render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}

async fn fire(
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

fn valid_time(v: &str) -> bool {
    matches!(v, "day" | "noon" | "sunset" | "night" | "midnight")
        || v.parse::<u32>().is_ok()
}

fn valid_weather(v: &str) -> bool {
    matches!(v, "clear" | "rain" | "thunder")
}

fn valid_difficulty(v: &str) -> bool {
    matches!(v, "peaceful" | "easy" | "normal" | "hard" | "0" | "1" | "2" | "3")
}

pub async fn post_time(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(value): Path<String>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !valid_time(&value) {
        return (StatusCode::BAD_REQUEST, "invalid time").into_response();
    }
    fire(
        &state,
        user.id,
        &user.name,
        "world.time",
        format!("/time set {value}"),
    )
    .await
}

pub async fn post_weather(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(value): Path<String>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !valid_weather(&value) {
        return (StatusCode::BAD_REQUEST, "invalid weather").into_response();
    }
    fire(
        &state,
        user.id,
        &user.name,
        "world.weather",
        format!("/weather {value}"),
    )
    .await
}

pub async fn post_difficulty(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(value): Path<String>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if !valid_difficulty(&value) {
        return (StatusCode::BAD_REQUEST, "invalid difficulty").into_response();
    }
    fire(
        &state,
        user.id,
        &user.name,
        "world.difficulty",
        format!("/difficulty {value}"),
    )
    .await
}
