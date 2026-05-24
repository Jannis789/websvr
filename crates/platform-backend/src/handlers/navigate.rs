use rama::http::Request;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common;
use crate::components::Shell;

static HOME_OVERVIEW_HTML: &str = include_str!("../../assets/fragments/content/overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../assets/fragments/content/movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../assets/fragments/content/series.html");

/// GET /home/overview
pub async fn get_home_overview(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    Shell::empty().content(HOME_OVERVIEW_HTML).emit(&ctx);
    common::empty_response(rama::http::StatusCode::SEE_OTHER)
}

/// GET /home/movies
pub async fn get_home_movies(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    Shell::empty().content(HOME_MOVIES_HTML).emit(&ctx);
    common::empty_response(rama::http::StatusCode::SEE_OTHER)
}

/// GET /home/series
pub async fn get_home_series(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    Shell::empty().content(HOME_SERIES_HTML).emit(&ctx);
    common::empty_response(rama::http::StatusCode::SEE_OTHER)
}
