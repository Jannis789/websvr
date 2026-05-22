use rama::http::{Request, Response, StatusCode};
use rama::http::body::sse::datastar::PatchElements;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common::{self, empty_response};
use crate::context::ClientContextSseExt;

static HOME_OVERVIEW_HTML: &str = include_str!("../../pages/home_overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../pages/home_movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../pages/home_series.html");

/// GET /home/overview — emit overview HTML via SSE
pub async fn get_home_overview(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    log!(Info, "Handler → get_home_overview (client_id={})", ctx.client_id);

    let patch = PatchElements::new(HOME_OVERVIEW_HTML.try_into().unwrap());
    ctx.emit_patch(HOME_OVERVIEW_HTML, patch, true);

    empty_response(StatusCode::NO_CONTENT)
}

/// GET /home/movies — emit movies HTML via SSE
pub async fn get_home_movies(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    log!(Info, "Handler → get_home_movies (client_id={})", ctx.client_id);

    let patch = PatchElements::new(HOME_MOVIES_HTML.try_into().unwrap());
    ctx.emit_patch(HOME_MOVIES_HTML, patch, true);

    empty_response(StatusCode::NO_CONTENT)
}

/// GET /home/series — emit series HTML via SSE
pub async fn get_home_series(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    log!(Info, "Handler → get_home_series (client_id={})", ctx.client_id);

    let patch = PatchElements::new(HOME_SERIES_HTML.try_into().unwrap());
    ctx.emit_patch(HOME_SERIES_HTML, patch, true);

    empty_response(StatusCode::NO_CONTENT)
}
