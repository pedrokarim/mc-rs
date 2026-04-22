//! `/api/snapshot` — JSON brut du snapshot courant (pour CLI/Grafana).

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;

use crate::state::AppState;

pub async fn api_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = crate::auth::middleware::require_auth(&state, &headers).await {
        return resp;
    }
    let snap = state.handle.snapshot.read().await;
    let uptime = snap.uptime_seconds();
    let mut val = serde_json::to_value(&*snap).unwrap_or(serde_json::Value::Null);
    if let serde_json::Value::Object(ref mut m) = val {
        m.insert("uptime_seconds".into(), serde_json::json!(uptime));
    }
    Json(val).into_response()
}
