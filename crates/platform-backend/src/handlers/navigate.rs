use rama::http::{Request, Response};
use rama::http::body::sse::datastar::PatchElements;
use rama::http::service::web::response::{Sse, IntoResponse};
use rama::http::service::web::extract::State;
use async_stream::stream;
use crate::server::SharedState;
use crate::common;

static HOME_OVERVIEW_HTML: &str = include_str!("../../assets/templates/home_overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../assets/templates/home_movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../assets/templates/home_series.html");

/// GET /home/overview — SSE response with PatchElements
pub async fn get_home_overview(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_overview (client_id={})", _ctx.client_id);
    render_fragment(HOME_OVERVIEW_HTML)
}

/// GET /home/movies — SSE response with PatchElements
pub async fn get_home_movies(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_movies (client_id={})", _ctx.client_id);
    render_fragment(HOME_MOVIES_HTML)
}

/// GET /home/series — SSE response with PatchElements
pub async fn get_home_series(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = common::extract_context(&req);
    tracing::debug!("Handler → get_home_series (client_id={})", _ctx.client_id);
    render_fragment(HOME_SERIES_HTML)
}

fn render_fragment(html: &str) -> Response {
    let patch = PatchElements::new(html.try_into().unwrap())
        .with_selector(".content-body".try_into().unwrap());

    let stream = stream! {
        match patch.try_into_sse_event() {
            Ok(event) => yield Ok::<_, rama::http::sse::EventBuildError>(event),
            Err(e) => {
                tracing::error!("Failed to build PatchElements SSE event: {e}");
            }
        }
    };

    Sse::new(stream).into_response()
}
