//! `GET /setup` + `POST /setup` — création du premier admin.
//!
//! Accessible sans auth UNIQUEMENT tant que `users` est vide. Après seed,
//! tout GET/POST sur `/setup` renvoie 404.

use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    http::StatusCode,
    Form,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::Role;
use crate::state::AppState;

#[derive(Template)]
#[template(path = "setup.html")]
struct SetupTemplate {
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct SetupForm {
    username: String,
    password: String,
    password_confirm: String,
}

pub async fn get_setup(State(state): State<Arc<AppState>>) -> Response {
    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };
    match db.user_count().await {
        Ok(0) => render(None).into_response(),
        Ok(_) => Redirect::to("/login").into_response(),
        Err(e) => {
            tracing::error!("[webui] setup: db error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response()
        }
    }
}

pub async fn post_setup(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SetupForm>,
) -> Response {
    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };
    match db.user_count().await {
        Ok(0) => {}
        Ok(_) => return Redirect::to("/login").into_response(),
        Err(e) => {
            tracing::error!("[webui] setup: db error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    }

    // Validation
    if form.username.trim().len() < 3 {
        return render(Some("Nom d'utilisateur trop court (min 3).")).into_response();
    }
    if form.password.len() < 8 {
        return render(Some("Mot de passe trop court (min 8).")).into_response();
    }
    if form.password != form.password_confirm {
        return render(Some("Les mots de passe ne correspondent pas.")).into_response();
    }

    let hash = match crate::auth::hash_password(&form.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("[webui] setup: hash error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "crypto error").into_response();
        }
    };

    let user = match db
        .create_user(form.username.trim(), &hash, Role::Admin)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("[webui] setup: create_user failed: {e}");
            return render(Some(&format!("Erreur création : {e}"))).into_response();
        }
    };

    let _ = db
        .audit_log(
            Some(user.id),
            Some(&user.username),
            "admin.setup",
            serde_json::json!({ "first_admin": true, "role": "admin" }),
        )
        .await;

    tracing::info!("[webui] first admin '{}' created via /setup", user.username);
    Redirect::to("/login").into_response()
}

fn render(error: Option<&str>) -> impl IntoResponse {
    let tpl = SetupTemplate {
        error: error.map(String::from),
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] setup template render failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}
