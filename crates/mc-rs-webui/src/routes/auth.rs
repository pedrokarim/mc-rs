//! `/login`, `/logout` — flux auth après le setup du 1er admin.

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn get_login(State(state): State<Arc<AppState>>) -> Response {
    // Si user vide : redirect setup
    if let Some(db) = state.db.clone() {
        if db.user_count().await.unwrap_or(1) == 0 {
            return Redirect::to("/setup").into_response();
        }
    }
    render_login(None)
}

pub async fn post_login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Form(form): Form<LoginForm>,
) -> Response {
    let (Some(db), Some(jwt)) = (state.db.clone(), state.jwt.clone()) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "auth subsystem not initialized").into_response();
    };

    // Rate limit par IP.
    if let Some(retry_after) = state.login_ratelimit.check_and_record(peer.ip()) {
        tracing::warn!(
            "[webui] login rate limit hit for {} — retry_after={}s",
            peer.ip(),
            retry_after
        );
        let _ = db
            .audit_log(
                None,
                Some(form.username.trim()),
                "auth.rate_limit",
                serde_json::json!({ "ip": peer.ip().to_string(), "retry_after": retry_after }),
            )
            .await;
        let mut resp = render_login(Some(&format!(
            "Trop de tentatives. Réessayez dans {retry_after}s."
        )));
        if let Ok(v) = retry_after.to_string().parse::<axum::http::HeaderValue>() {
            resp.headers_mut().insert("Retry-After", v);
        }
        *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        return resp;
    }

    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let user = match db.find_user_by_name(form.username.trim()).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Timing-constant : on vérifie quand même un hash fictif pour ne pas
            // révéler si l'user existe ou non via le temps de réponse.
            let _ = crate::auth::verify_password(
                &form.password,
                "$argon2id$v=19$m=19456,t=2,p=1$ZmFrZXNhbHRmYWtlc2FsdA$fakehashfakehashfakehashfakehashfakehashfakehashfakehash",
            );
            let _ = db
                .audit_log(
                    None,
                    Some(form.username.trim()),
                    "auth.login_failed",
                    serde_json::json!({ "reason": "unknown_user", "ip": peer.ip().to_string(), "ua": ua }),
                )
                .await;
            return render_login(Some("Identifiants invalides."));
        }
        Err(e) => {
            tracing::error!("[webui] login: db error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let ok = match crate::auth::verify_password(&form.password, &user.password_hash) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[webui] login: verify error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "crypto error").into_response();
        }
    };
    if !ok {
        let _ = db
            .audit_log(
                Some(user.id),
                Some(&user.username),
                "auth.login_failed",
                serde_json::json!({ "reason": "bad_password", "ip": peer.ip().to_string(), "ua": ua }),
            )
            .await;
        return render_login(Some("Identifiants invalides."));
    }

    // Succès : reset le compteur de l'IP.
    state.login_ratelimit.reset(peer.ip());

    let (token, claims) = match jwt.issue(user.id, &user.username, user.role) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("[webui] login: jwt issue error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "jwt error").into_response();
        }
    };

    let now = chrono::Utc::now().timestamp();
    let _ = db.touch_login(user.id, now).await;
    let _ = db
        .audit_log(
            Some(user.id),
            Some(&user.username),
            "auth.login",
            serde_json::json!({ "ok": true, "jti": claims.jti }),
        )
        .await;

    let max_age = (claims.exp - now).max(0);
    let secure = state.config.tls.enabled;
    let cookie = format!(
        "auth={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
        if secure { "; Secure" } else { "" }
    );

    let mut resp = Redirect::to("/").into_response();
    resp.headers_mut().insert(
        SET_COOKIE,
        cookie.parse().expect("well-formed cookie header"),
    );
    resp
}

pub async fn post_logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    if let Some(db) = state.db.clone() {
        let _ = db.blacklist_token(&user.jti, user.exp).await;
        let _ = db
            .audit_log(
                Some(user.id),
                Some(&user.name),
                "auth.logout",
                serde_json::json!({ "jti": user.jti }),
            )
            .await;
    }

    let mut resp = Redirect::to("/login").into_response();
    resp.headers_mut().insert(
        SET_COOKIE,
        "auth=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"
            .parse()
            .expect("well-formed cookie header"),
    );
    resp
}

fn render_login(error: Option<&str>) -> Response {
    let tpl = LoginTemplate {
        error: error.map(String::from),
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] login template render failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}
