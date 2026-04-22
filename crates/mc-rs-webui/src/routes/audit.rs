//! `/audit` — vue paginée de la table `audit_log`.

use askama::Template;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::AuditFilter;
use crate::state::AppState;

const PAGE_SIZE: u32 = 50;

#[derive(Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    offset: u32,
}

struct AuditRow {
    ts_str: String,
    username: String,
    action: String,
    detail_str: String,
}

#[derive(Template)]
#[template(path = "audit.html")]
struct AuditTemplate {
    current_page: &'static str,
    user_name: String,
    user_role: String,
    entries: Vec<AuditRow>,
    total: u64,
    offset: u32,
    prev_offset: u32,
    next_offset: u32,
    has_next: bool,
}

pub async fn get_audit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<AuditQuery>,
) -> Response {
    let user = match crate::auth::middleware::require_auth(&state, &headers).await {
        Ok(u) => u,
        Err(resp) => return resp,
    };

    let Some(db) = state.db.clone() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "db not initialized").into_response();
    };

    let filter = AuditFilter::default();
    let total = db.audit_count(filter.clone()).await.unwrap_or(0);
    let rows = match db.audit_page(PAGE_SIZE, q.offset, filter).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("[webui] audit_page: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let entries: Vec<AuditRow> = rows
        .into_iter()
        .map(|e| AuditRow {
            ts_str: chrono::DateTime::<chrono::Local>::from(
                chrono::DateTime::from_timestamp(e.ts, 0).unwrap_or_default(),
            )
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
            username: e.username_snapshot.unwrap_or_else(|| "(système)".to_string()),
            action: e.action,
            detail_str: e.detail.to_string(),
        })
        .collect();

    let has_next = (q.offset as u64 + PAGE_SIZE as u64) < total;
    let prev_offset = q.offset.saturating_sub(PAGE_SIZE);
    let next_offset = q.offset + PAGE_SIZE;

    let tpl = AuditTemplate {
        current_page: "audit",
        user_name: user.name,
        user_role: user.role.as_str().to_string(),
        entries,
        total,
        offset: q.offset,
        prev_offset,
        next_offset,
        has_next,
    };
    match tpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[webui] audit render: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "template error").into_response()
        }
    }
}
