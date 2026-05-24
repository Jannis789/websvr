use rama::http::{Request, StatusCode};
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common;
use crate::components::Shell;

static HOME_OVERVIEW_HTML: &str = include_str!("../../assets/fragments/content/overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../assets/fragments/content/movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../assets/fragments/content/series.html");

/// GET /home/overview — emit PatchElements via SSE broadcaster, return 204
pub async fn get_home_overview(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_overview (client_id={})", ctx.client_id);

    Shell::empty()
        .content(HOME_OVERVIEW_HTML)
        .emit(&ctx);

    common::empty_response(StatusCode::NO_CONTENT)
}

/// GET /home/movies — emit PatchElements via SSE broadcaster, return 204
pub async fn get_home_movies(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_movies (client_id={})", ctx.client_id);

    Shell::empty()
        .content(HOME_MOVIES_HTML)
        .emit(&ctx);

    common::empty_response(StatusCode::NO_CONTENT)
}

/// GET /home/series — emit PatchElements via SSE broadcaster, return 204
pub async fn get_home_series(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_series (client_id={})", ctx.client_id);

    Shell::empty()
        .content(HOME_SERIES_HTML)
        .emit(&ctx);

    common::empty_response(StatusCode::NO_CONTENT)
}
