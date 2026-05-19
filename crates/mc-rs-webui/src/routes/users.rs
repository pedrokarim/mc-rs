//! `/users` — CRUD des comptes admin du panel. Accessible uniquement aux `admin`.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::Role;
use crate::state::AppState;

struct UserRow {
    id_str: String,
    username: String,
    role: String,
    created_str: String,
    last_login_str: String,
    is_self: bool,
}

#[derive(Template)]
#[template(path = "users.html")]
struct UsersTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
    users: Vec<UserRow>,
    error: Option<String>,
    success: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct UsersQuery {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    success: Option<String>,
}

fn ts_to_str(ts: i64) -> String {
    if ts == 0 {
        return "—".to_string();
    }
    match chrono::DateTime::from_timestamp(ts, 0) {
        Some(dt) => chrono::DateTime::<chrono::Local>::from(dt)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
        None => "—".to_string(),
    }
}

pub async fn get_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<UsersQuery>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if user.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "admin only").into_response();
    }

    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };

    let users = match db.list_users().await {
        Ok(us) => us,
        Err(e) => {
            tracing::error!("[webui] list_users: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let rows: Vec<UserRow> = users
        .into_iter()
        .map(|u| UserRow {
            id_str: u.id.to_string(),
            is_self: u.id == user.id,
            username: u.username,
            role: u.role.as_str().to_string(),
            created_str: ts_to_str(u.created_at),
            last_login_str: ts_to_str(u.last_login_at.unwrap_or(0)),
        })
        .collect();

    let tpl = UsersTemplate {
        current_page: "users",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
        users: rows,
        error: q.error,
        success: q.success,
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] users render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUserForm {
    username: String,
    password: String,
    role: String,
}

async fn ensure_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::auth::CurrentUser, Response> {
    let user = crate::auth::middleware::require_auth(state, headers).await?;
    if user.role != Role::Admin {
        return Err((StatusCode::FORBIDDEN, "admin only").into_response());
    }
    Ok(user)
}

fn redirect_with(param: &str, value: &str) -> Response {
    let url = format!("/users?{}={}", param, urlencoded(value));
    Redirect::to(&url).into_response()
}

fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            out.push(c);
        } else {
            for b in c.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

pub async fn post_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<CreateUserForm>,
) -> Response {
    let user = match ensure_admin(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };

    let username = form.username.trim();
    if username.len() < 3 || username.len() > 32 {
        return redirect_with("error", "Nom invalide (3-32 caractères)");
    }
    if form.password.len() < 8 {
        return redirect_with("error", "Mot de passe trop court (min 8)");
    }
    let Some(role) = Role::from_str(&form.role) else {
        return redirect_with("error", "Rôle invalide");
    };

    if db
        .find_user_by_name(username)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return redirect_with("error", "Ce nom est déjà pris");
    }

    let hash = match crate::auth::hash_password(&form.password) {
        Ok(h) => h,
        Err(e) => return redirect_with("error", &format!("Erreur crypto : {e}")),
    };

    let created = match db.create_user(username, &hash, role).await {
        Ok(u) => u,
        Err(e) => return redirect_with("error", &format!("Erreur DB : {e}")),
    };

    let _ = db
        .audit_log(
            Some(user.id),
            Some(&user.name),
            "users.create",
            serde_json::json!({ "username": username, "role": role.as_str() }),
        )
        .await;
    tracing::info!(
        "[webui] user '{}' created by '{}' ({})",
        created.username,
        user.name,
        role.as_str()
    );
    redirect_with(
        "success",
        &format!("Utilisateur '{}' créé", created.username),
    )
}

#[derive(Deserialize)]
pub struct PasswordForm {
    password: String,
}

pub async fn post_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Form(form): Form<PasswordForm>,
) -> Response {
    let user = match ensure_admin(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };
    let Ok(id) = Uuid::parse_str(&id_str) else {
        return redirect_with("error", "ID invalide");
    };
    if form.password.len() < 8 {
        return redirect_with("error", "Mot de passe trop court");
    }
    let hash = match crate::auth::hash_password(&form.password) {
        Ok(h) => h,
        Err(e) => return redirect_with("error", &format!("Erreur crypto : {e}")),
    };
    if let Err(e) = db.update_password(id, &hash).await {
        return redirect_with("error", &format!("Erreur DB : {e}"));
    }
    let _ = db
        .audit_log(
            Some(user.id),
            Some(&user.name),
            "users.password_reset",
            serde_json::json!({ "target_id": id.to_string() }),
        )
        .await;
    redirect_with("success", "Mot de passe mis à jour")
}

#[derive(Deserialize)]
pub struct RoleForm {
    role: String,
}

pub async fn post_role(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Form(form): Form<RoleForm>,
) -> Response {
    let user = match ensure_admin(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };
    let Ok(id) = Uuid::parse_str(&id_str) else {
        return redirect_with("error", "ID invalide");
    };
    let Some(role) = Role::from_str(&form.role) else {
        return redirect_with("error", "Rôle invalide");
    };

    // Protection : ne pas laisser un admin se rétrograder si c'est le dernier.
    if id == user.id && role != Role::Admin {
        let admins = db.list_users().await.unwrap_or_default();
        let admin_count = admins.iter().filter(|u| u.role == Role::Admin).count();
        if admin_count <= 1 {
            return redirect_with("error", "Impossible : vous êtes le dernier admin");
        }
    }

    if let Err(e) = db.update_role(id, role).await {
        return redirect_with("error", &format!("Erreur DB : {e}"));
    }
    let _ = db
        .audit_log(
            Some(user.id),
            Some(&user.name),
            "users.role_change",
            serde_json::json!({ "target_id": id.to_string(), "new_role": role.as_str() }),
        )
        .await;
    redirect_with("success", "Rôle mis à jour")
}

pub async fn post_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
) -> Response {
    let user = match ensure_admin(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };
    let Ok(id) = Uuid::parse_str(&id_str) else {
        return redirect_with("error", "ID invalide");
    };
    if id == user.id {
        return redirect_with("error", "Impossible : c'est votre compte");
    }

    // Lookup du target pour audit (nom après suppression serait perdu)
    let target_name = db
        .list_users()
        .await
        .ok()
        .and_then(|users| users.into_iter().find(|u| u.id == id).map(|u| u.username))
        .unwrap_or_else(|| "(inconnu)".to_string());

    if let Err(e) = db.delete_user(id).await {
        return redirect_with("error", &format!("Erreur DB : {e}"));
    }
    let _ = db
        .audit_log(
            Some(user.id),
            Some(&user.name),
            "users.delete",
            serde_json::json!({ "target_id": id.to_string(), "target_name": target_name }),
        )
        .await;
    redirect_with("success", "Utilisateur supprimé")
}
