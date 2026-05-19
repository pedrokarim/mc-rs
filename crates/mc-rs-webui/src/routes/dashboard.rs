//! `GET /` — dashboard principal.

use askama::Template;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
    world_name: String,
    max_players: u32,
}

pub async fn get_dashboard(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let snap = state.handle.snapshot.read().await;
    let tpl = DashboardTemplate {
        current_page: "dashboard",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
        world_name: snap.world_name.clone(),
        max_players: snap.max_players,
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] dashboard render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}
