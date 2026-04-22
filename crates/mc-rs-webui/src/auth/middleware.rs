//! Helper d'authentification. Appelé au début de chaque handler protégé pour
//! extraire l'`CurrentUser` depuis le cookie JWT + vérifier la blacklist DB.
//!
//! Pourquoi pas un `middleware::from_fn` axum ? Axum 0.7 a une inférence de
//! types très stricte sur les extracteurs de middleware — utiliser un helper
//! évite ce piège sans sacrifier la sécurité (chaque handler protégé appelle
//! `require_auth` en première ligne).

use axum::{
    http::{
        header::{ACCEPT, COOKIE},
        StatusCode,
    },
    response::{IntoResponse, Redirect, Response},
};
use axum::http::HeaderMap;
use uuid::Uuid;

use crate::db::Role;
use crate::state::AppState;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
    pub jti: String,
    pub exp: i64,
}

/// À appeler en entrée de handler protégé. Si OK, renvoie le `CurrentUser`.
/// Si KO, renvoie une `Response` à propager directement (redirect HTML ou 401 JSON).
pub async fn require_auth(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<CurrentUser, Response> {
    match authenticate(state, headers).await {
        Ok(u) => Ok(u),
        Err(AuthError::Missing) | Err(AuthError::Invalid) => {
            if wants_html(headers) {
                Err(Redirect::to("/login").into_response())
            } else {
                Err((StatusCode::UNAUTHORIZED, "unauthorized").into_response())
            }
        }
        Err(AuthError::ServerError(msg)) => {
            tracing::error!("[webui] auth error: {}", msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response())
        }
    }
}

#[derive(Debug)]
enum AuthError {
    Missing,
    Invalid,
    ServerError(String),
}

async fn authenticate(state: &AppState, headers: &HeaderMap) -> Result<CurrentUser, AuthError> {
    let cookie_header = headers
        .get(COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::Missing)?;

    let token = extract_cookie(cookie_header, "auth").ok_or(AuthError::Missing)?;

    let codec = state
        .jwt
        .as_ref()
        .ok_or_else(|| AuthError::ServerError("jwt codec not initialized".to_string()))?;
    let claims = codec.decode(&token).map_err(|_| AuthError::Invalid)?;

    let db = state
        .db
        .as_ref()
        .ok_or_else(|| AuthError::ServerError("db not initialized".to_string()))?;
    let blacklisted = db
        .is_blacklisted(&claims.jti)
        .await
        .map_err(|e| AuthError::ServerError(e.to_string()))?;
    if blacklisted {
        return Err(AuthError::Invalid);
    }

    let id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::Invalid)?;
    let role = Role::from_str(&claims.role).ok_or(AuthError::Invalid)?;

    Ok(CurrentUser {
        id,
        name: claims.name,
        role,
        jti: claims.jti,
        exp: claims.exp,
    })
}

fn extract_cookie(header: &str, name: &str) -> Option<String> {
    for part in header.split(';') {
        let part = part.trim();
        if let Some(eq) = part.find('=') {
            let (k, v) = part.split_at(eq);
            if k.eq_ignore_ascii_case(name) {
                return Some(v[1..].to_string());
            }
        }
    }
    None
}

fn wants_html(headers: &HeaderMap) -> bool {
    headers
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/html"))
        .unwrap_or(false)
}
