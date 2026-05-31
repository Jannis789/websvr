use crate::context::SharedState;
use rama::http::service::web::extract::State;
use rama::http::Request;

use crate::components::Shell;
use crate::utils::request::extract_context;

static HOME_OVERVIEW_HTML: &str = include_str!("../../assets/fragments/content/overview.html");
static HOME_MOVIES_HTML: &str = include_str!("../../assets/fragments/content/movies.html");
static HOME_SERIES_HTML: &str = include_str!("../../assets/fragments/content/series.html");

const I18N_KEYS_OVERVIEW: &[&str] = &["nav_overview", "content_overview"];
const I18N_KEYS_MOVIES: &[&str] = &["nav_movies", "content_movies"];
const I18N_KEYS_SERIES: &[&str] = &["nav_series", "content_series"];

/// GET /home/overview
pub async fn get_home_overview(
    State(state): State<SharedState>,
    req: Request,
) -> crate::utils::response::Response {
    let ctx = extract_context(&req);
    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS_OVERVIEW);
    Shell::empty()
        .content(HOME_OVERVIEW_HTML)
        .signals(&i18n_signals)
        .emit_response(&ctx)
}

/// GET /home/movies
pub async fn get_home_movies(
    State(state): State<SharedState>,
    req: Request,
) -> crate::utils::response::Response {
    let ctx = extract_context(&req);
    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS_MOVIES);
    Shell::empty()
        .content(HOME_MOVIES_HTML)
        .signals(&i18n_signals)
        .emit_response(&ctx)
}

/// GET /home/series
pub async fn get_home_series(
    State(state): State<SharedState>,
    req: Request,
) -> crate::utils::response::Response {
    let ctx = extract_context(&req);
    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS_SERIES);
    Shell::empty()
        .content(HOME_SERIES_HTML)
        .signals(&i18n_signals)
        .emit_response(&ctx)
}
