use rama::http::{Request, Response, StatusCode};
use rama::http::header;
use rama::http::service::web::extract::State;
use crate::server::SharedState;

/// GET /icons/{name}.svg — serve GNOME Icon Development Kit SVGs
///
/// Each icon is compile-time embedded via `include_str!`.
/// To add a new icon: copy the SVG to `assets/icons/`, add a match arm below.
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

    let svg = match name {
        "dark-mode" => include_str!("../../assets/icons/dark-mode.svg"),
        "display" => include_str!("../../assets/icons/display.svg"),
        "exit" => include_str!("../../assets/icons/exit.svg"),
        "go-home" => include_str!("../../assets/icons/go-home.svg"),
        "moon" => include_str!("../../assets/icons/moon.svg"),
        "multimedia" => include_str!("../../assets/icons/multimedia.svg"),
        "open-menu" => include_str!("../../assets/icons/open-menu.svg"),
        "padlock-closed" => include_str!("../../assets/icons/padlock-closed.svg"),
        "padlock-open" => include_str!("../../assets/icons/padlock-open.svg"),
        "person" => include_str!("../../assets/icons/person.svg"),
        "shield" => include_str!("../../assets/icons/shield.svg"),
        "star" => include_str!("../../assets/icons/star.svg"),
        "sun" => include_str!("../../assets/icons/sun.svg"),
        "tv" => include_str!("../../assets/icons/tv.svg"),
        "video" => include_str!("../../assets/icons/video.svg"),
        "view-grid" => include_str!("../../assets/icons/view-grid.svg"),
        "view-list" => include_str!("../../assets/icons/view-list.svg"),
        "wrench" => include_str!("../../assets/icons/wrench.svg"),
        _ => return not_found(),
    };

    let mut resp = Response::new(rama::http::Body::from(svg));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "image/svg+xml".parse().expect("invalid header value"),
    );
    resp
}

fn not_found() -> Response {
    let mut resp = Response::new(rama::http::Body::from("Not Found"));
    *resp.status_mut() = StatusCode::NOT_FOUND;
    resp
}
