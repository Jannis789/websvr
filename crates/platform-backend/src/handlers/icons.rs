use rama::http::{Request, Response, StatusCode};
use rama::http::header;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use std::path::PathBuf;

/// GET /icons/{name}.svg — serve GNOME Icon Development Kit SVGs from assets/icons/
pub async fn icon_handler(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let path = req.uri().path();
    let name = path
        .strip_prefix("/icons/")
        .and_then(|s| s.strip_suffix(".svg"))
        .unwrap_or("");

    // Sanitize: only allow [a-zA-Z0-9_-]
    if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') || name.is_empty() {
        return not_found();
    }

    // Path traversal protection
    if name.contains("..") {
        return not_found();
    }

    let icon_path: PathBuf = format!("assets/icons/{name}.svg").into();

    match std::fs::read_to_string(&icon_path) {
        Ok(svg) => {
            let mut resp = Response::new(rama::http::Body::from(svg));
            *resp.status_mut() = StatusCode::OK;
            resp.headers_mut().insert(
                header::CONTENT_TYPE,
                "image/svg+xml".parse().expect("invalid header value"),
            );
            resp
        }
        Err(_) => not_found(),
    }
}

fn not_found() -> Response {
    let mut resp = Response::new(rama::http::Body::from("Not Found"));
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}
