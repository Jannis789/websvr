use rama::http::{Request, Response, StatusCode};
use rama::http::header;
use rama::http::service::web::extract::State;
use crate::server::SharedState;

/// GET /icons/{name}.svg — serve SVG icons
///
/// In a later phase, icons will be embedded via `include_dir!`.
/// For now, returns a simple 200 with a placeholder SVG.
pub async fn icon_handler(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let path = req.uri().path();
    let icon_name = path
        .strip_prefix("/icons/")
        .and_then(|s| s.strip_suffix(".svg"))
        .unwrap_or("unknown");

    let svg = placeholder_svg(icon_name);
    let mut resp = Response::new(rama::http::Body::from(svg));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "image/svg+xml".parse().expect("invalid header value"),
    );
    resp
}

fn placeholder_svg(name: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><text x="12" y="16" text-anchor="middle" font-size="8" fill="currentColor">{}</text></svg>"#,
        name.chars().take(2).collect::<String>()
    )
}
