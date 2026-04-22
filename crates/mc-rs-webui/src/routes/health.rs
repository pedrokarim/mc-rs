//! `GET /api/health` — ping non authentifié. Retourne toujours 200.

use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::state::AppState;

pub async fn get_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let snap = state.handle.snapshot.read().await;
    Json(json!({
        "ok": true,
        "uptime_seconds": snap.uptime_seconds(),
        "players_online": snap.players.len(),
        "tps": snap.tps,
    }))
}
