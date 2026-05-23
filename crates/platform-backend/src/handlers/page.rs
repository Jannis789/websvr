use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use rama::http::header;
use crate::server::SharedState;
use crate::common::{self, html_response};

/// GET /login — login page
pub async fn login_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    tracing::debug!("Handler → login_page (public route)");
    html_response(include_str!("../../assets/templates/login.html"))
}

/// GET /register — registration page
pub async fn register_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    tracing::debug!("Handler → register_page (public route)");
    html_response(include_str!("../../assets/templates/register.html"))
}

/// GET /home — main application shell
pub async fn home_page(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = common::extract_context(&req);
    tracing::debug!("Handler → home_page (client_id={})", _ctx.client_id);
    html_response(include_str!("../../assets/templates/home.html"))
}

// ── Asset handlers (public, no layer stack) ──

/// GET /sw.js — Service Worker
pub async fn service_worker(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    let mut resp = Response::new(rama::http::Body::from(
        include_str!("../../assets/js/sw.js")
    ));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/javascript".parse().expect("invalid header value"),
    );
    resp
}

/// GET /assets/css/dark.css
pub async fn asset_dark_css(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    css_response(include_str!("../../assets/css/dark.css"))
}

/// GET /assets/css/light.css
pub async fn asset_light_css(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    css_response(include_str!("../../assets/css/light.css"))
}

/// GET /assets/css/common.css
pub async fn asset_common_css(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    css_response(include_str!("../../assets/css/common.css"))
}

/// GET /assets/css/{name}.css — Generic CSS asset handler
///
/// Serves any known CSS file from the `assets/css/` directory tree.
/// Uses compile-time `include_str!` embedding — each new CSS file must be
/// added as a match arm below.
pub async fn asset_css(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let path = req.uri().path();
    // path is like /assets/css/home.css or /assets/css/features/_theme.css
    let relative = path.strip_prefix("/assets/css/").unwrap_or(path);

    let content = match relative {
        // ── Top-level entry points ──
        "home.css" => include_str!("../../assets/css/home.css"),
        "auth.css" => include_str!("../../assets/css/auth.css"),
        "test.css" => include_str!("../../assets/css/test.css"),
        // ── Feature partials (features/) ──
        "features/_theme.css" => include_str!("../../assets/css/features/_theme.css"),
        "features/_base.css" => include_str!("../../assets/css/features/_base.css"),
        "features/_window.css" => include_str!("../../assets/css/features/_window.css"),
        "features/_sidebar.css" => include_str!("../../assets/css/features/_sidebar.css"),
        "features/_popup.css" => include_str!("../../assets/css/features/_popup.css"),
        "features/_switch.css" => include_str!("../../assets/css/features/_switch.css"),
        "features/_content.css" => include_str!("../../assets/css/features/_content.css"),
        "features/_buttons.css" => include_str!("../../assets/css/features/_buttons.css"),
        "features/_forms.css" => include_str!("../../assets/css/features/_forms.css"),
        "features/_cards.css" => include_str!("../../assets/css/features/_cards.css"),
        "features/_test.css" => include_str!("../../assets/css/features/_test.css"),
        "features/_utilities.css" => include_str!("../../assets/css/features/_utilities.css"),
        _ => {
            let mut resp = Response::new(rama::http::Body::from("Not Found"));
            *resp.status_mut() = StatusCode::NOT_FOUND;
            return resp;
        }
    };
    css_response(content)
}

/// GET /assets/js/datastar-core.js — Datastar core script
pub async fn asset_datastar_core(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    let mut resp = Response::new(rama::http::Body::from(
        include_str!("../../assets/js/datastar-core.js")
    ));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/javascript".parse().expect("invalid header value"),
    );
    resp
}

fn css_response(css: &str) -> Response {
    let mut resp = Response::new(rama::http::Body::from(css.to_string()));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/css".parse().expect("invalid header value"),
    );
    resp
}
