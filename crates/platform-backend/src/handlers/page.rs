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
