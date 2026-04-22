//! Composition du Router axum.
//!
//! Routes publiques : `/api/health`, `/static/*`, `/setup`, `/login`.
//! Routes protégées : tout le reste — auth check inline via `require_auth`.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::state::AppState;

mod audit;
mod auth;
mod config;
mod console;
mod dashboard;
mod health;
mod home;
mod players;
mod setup;
mod statics;
mod users;
mod world;
mod ws;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Public
        .route("/api/health", get(health::get_health))
        .route("/static/*path", get(statics::get_asset))
        .route("/setup", get(setup::get_setup).post(setup::post_setup))
        .route("/login", get(auth::get_login).post(auth::post_login))
        // Protected
        .route("/", get(dashboard::get_dashboard))
        .route("/api/snapshot", get(home::api_snapshot))
        .route("/logout", post(auth::post_logout))
        .route("/console", get(console::get_console))
        .route("/console/execute", post(console::post_execute))
        .route("/players", get(players::get_players))
        .route("/players/:name/kick", post(players::post_kick))
        .route("/players/:name/op", post(players::post_op))
        .route("/players/:name/deop", post(players::post_deop))
        .route("/players/:name/gamemode", post(players::post_gamemode))
        .route("/world", get(world::get_world))
        .route("/world/time/:value", post(world::post_time))
        .route("/world/weather/:value", post(world::post_weather))
        .route("/world/difficulty/:value", post(world::post_difficulty))
        .route("/config", get(config::get_config).post(config::post_config))
        .route("/audit", get(audit::get_audit))
        .route("/users", get(users::get_users))
        .route("/users/create", post(users::post_create))
        .route("/users/:id/password", post(users::post_password))
        .route("/users/:id/role", post(users::post_role))
        .route("/users/:id/delete", post(users::post_delete))
        .route("/ws/logs", get(ws::ws_logs))
        .route("/ws/events", get(ws::ws_events))
        .route("/ws/snapshot", get(ws::ws_snapshot))
        .with_state(state)
}
