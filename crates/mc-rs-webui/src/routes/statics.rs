//! `GET /static/*path` — sert les assets embarqués (css, htmx, alpine).

use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub async fn get_asset(Path(path): Path<String>) -> Response {
    match Assets::get(&path) {
        Some(file) => {
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let mut resp = file.data.into_response();
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                mime.parse().unwrap_or(header::HeaderValue::from_static("application/octet-stream")),
            );
            // Cache 1h — si on modifie app.css il faut hard-reload mais c'est admin-only.
            resp.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("public, max-age=3600"),
            );
            resp
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
