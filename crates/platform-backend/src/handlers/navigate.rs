use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use rama::http::body::sse::datastar::PatchElements;
use crate::server::SharedState;
use crate::common;
use crate::context::ClientContextSseExt;

static HOME_OVERVIEW_HTML: &str = include_str!("../../assets/templates/home_overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../assets/templates/home_movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../assets/templates/home_series.html");

/// GET /home/overview — emit PatchElements via SSE broadcaster, return 204
pub async fn get_home_overview(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_overview (client_id={})", ctx.client_id);

    let patch = PatchElements::new(HOME_OVERVIEW_HTML.try_into().unwrap())
        .with_selector(".content-body".try_into().unwrap());

    ctx.emit_patch(HOME_OVERVIEW_HTML, patch, true);

    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(rama::http::Body::empty())
        .unwrap()
}

/// GET /home/movies — emit PatchElements via SSE broadcaster, return 204
pub async fn get_home_movies(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_movies (client_id={})", ctx.client_id);

    let patch = PatchElements::new(HOME_MOVIES_HTML.try_into().unwrap())
        .with_selector(".content-body".try_into().unwrap());

    ctx.emit_patch(HOME_MOVIES_HTML, patch, true);

    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(rama::http::Body::empty())
        .unwrap()
}

/// GET /home/series — emit PatchElements via SSE broadcaster, return 204
pub async fn get_home_series(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_series (client_id={})", ctx.client_id);

    let patch = PatchElements::new(HOME_SERIES_HTML.try_into().unwrap())
        .with_selector(".content-body".try_into().unwrap());

    ctx.emit_patch(HOME_SERIES_HTML, patch, true);

    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(rama::http::Body::empty())
        .unwrap()
}
