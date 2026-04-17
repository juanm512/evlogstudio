use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use include_dir::{include_dir, Dir};

static UI_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/static_ui");

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match UI_DIR.get_file(path) {
        Some(file) => {
            let mime = mime_type(path);
            ([(header::CONTENT_TYPE, mime)], file.contents()).into_response()
        }
        None => {
            // SPA fallback — siempre retornar index.html para rutas del cliente
            match UI_DIR.get_file("index.html") {
                Some(file) => (
                    [(header::CONTENT_TYPE, "text/html")],
                    file.contents()
                ).into_response(),
                None => StatusCode::NOT_FOUND.into_response()
            }
        }
    }
}

fn mime_type(path: &str) -> &'static str {
    if path.ends_with(".html") { "text/html" }
    else if path.ends_with(".js") { "application/javascript" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".ico") { "image/x-icon" }
    else if path.ends_with(".woff2") { "font/woff2" }
    else { "application/octet-stream" }
}
