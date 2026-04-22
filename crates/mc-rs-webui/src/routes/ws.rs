//! WebSocket endpoints — logs, events, snapshot push.
//!
//! - `/ws/logs`     : stream chaque LogLine émise par le tracing layer
//! - `/ws/events`   : stream chaque WebEvent émis par le serveur
//! - `/ws/snapshot` : push le ServerSnapshot à chaque update (~20 Hz)
//!
//! Auth : vérifiée via cookie JWT au moment du handshake HTTP. Une fois
//! upgradée, la connexion reste ouverte jusqu'à déconnexion client.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::HeaderMap,
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;

pub async fn ws_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = crate::auth::middleware::require_auth(&state, &headers).await {
        return resp;
    }
    let rx = state.handle.log_tx.subscribe();
    ws.on_upgrade(move |socket| logs_task(socket, rx))
}

pub async fn ws_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = crate::auth::middleware::require_auth(&state, &headers).await {
        return resp;
    }
    let rx = state.handle.event_tx.subscribe();
    ws.on_upgrade(move |socket| events_task(socket, rx))
}

pub async fn ws_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = crate::auth::middleware::require_auth(&state, &headers).await {
        return resp;
    }
    let snapshot = state.handle.snapshot.clone();
    ws.on_upgrade(move |socket| snapshot_task(socket, snapshot))
}

async fn logs_task(socket: WebSocket, mut rx: tokio::sync::broadcast::Receiver<crate::LogLine>) {
    let (mut sender, mut receiver) = socket.split();
    loop {
        tokio::select! {
            // Client a fermé la connexion ?
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => continue,
                }
            }
            msg = rx.recv() => {
                match msg {
                    Ok(line) => {
                        if let Ok(json) = serde_json::to_string(&line) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn events_task(
    socket: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<crate::WebEvent>,
) {
    let (mut sender, mut receiver) = socket.split();
    loop {
        tokio::select! {
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => continue,
                }
            }
            msg = rx.recv() => {
                match msg {
                    Ok(ev) => {
                        if let Ok(json) = serde_json::to_string(&ev) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn snapshot_task(
    socket: WebSocket,
    snapshot: Arc<tokio::sync::RwLock<crate::ServerSnapshot>>,
) {
    let (mut sender, mut receiver) = socket.split();
    // Push à 20 Hz — même fréquence que l'update serveur, pas d'aliasing.
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    loop {
        tokio::select! {
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => continue,
                }
            }
            _ = tick.tick() => {
                let snap_json = {
                    let snap = snapshot.read().await;
                    // On sérialise un objet enrichi avec uptime_seconds calculé.
                    let uptime = snap.uptime_seconds();
                    let mut val = serde_json::to_value(&*snap).unwrap_or(serde_json::Value::Null);
                    if let serde_json::Value::Object(ref mut m) = val {
                        m.insert("uptime_seconds".into(), serde_json::json!(uptime));
                    }
                    val.to_string()
                };
                if sender.send(Message::Text(snap_json.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
