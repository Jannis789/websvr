use rama::http::Request;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common;

static HOME_OVERVIEW_HTML: &str = include_str!("../../assets/fragments/content/overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../assets/fragments/content/movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../assets/fragments/content/series.html");

/// GET /home/overview — return SSE patch directly in response
pub async fn get_home_overview(
    State(_state): State<SharedState>,
    _req: Request,
) -> common::Response {
    common::sse_patch_response("#content-body", HOME_OVERVIEW_HTML)
}

/// GET /home/movies — return SSE patch directly in response
pub async fn get_home_movies(
    State(_state): State<SharedState>,
    _req: Request,
) -> common::Response {
    common::sse_patch_response("#content-body", HOME_MOVIES_HTML)
}

/// GET /home/series — return SSE patch directly in response
pub async fn get_home_series(
    State(_state): State<SharedState>,
    _req: Request,
) -> common::Response {
    common::sse_patch_response("#content-body", HOME_SERIES_HTML)
}
