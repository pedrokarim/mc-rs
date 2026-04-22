//! `/console` — page terminal web + `POST /console/execute`.

use askama::Template;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Template)]
#[template(path = "console.html")]
struct ConsoleTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
}

pub async fn get_console(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let tpl = ConsoleTemplate {
        current_page: "console",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] console render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ExecuteForm {
    command: String,
}

pub async fn post_execute(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<ExecuteForm>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let cmd = form.command.trim();
    if cmd.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty command").into_response();
    }

    // Injection dans la main loop : le console_rx pioche ça au prochain tick.
    if state.handle.console_tx.send(cmd.to_string()).is_err() {
        return (StatusCode::SERVICE_UNAVAILABLE, "server console closed").into_response();
    }

    // Audit + event broadcast.
    if let Some(db) = state.db.clone() {
        let _ = db
            .audit_log(
                Some(user.id),
                Some(&user.name),
                "console.execute",
                serde_json::json!({ "command": cmd }),
            )
            .await;
    }
    let _ = state.handle.event_tx.send(crate::WebEvent::AdminAction {
        actor: user.name.clone(),
        action: "console.execute".to_string(),
        detail: cmd.to_string(),
    });

    (StatusCode::OK, "queued").into_response()
}
