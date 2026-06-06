use crate::context::SharedState;
use crate::ui::constants::{elements, signals};
use rama::http::service::web::extract::State;
use rama::http::Request;

use crate::components::Shell;
use crate::utils::request::extract_context;
use crate::utils::response::sse_response;

const I18N_KEYS_OVERVIEW: &[&str] = &["nav_overview", "content_overview"].as_slice();
const I18N_KEYS_MOVIES: &[&str] = &["nav_movies", "content_movies"].as_slice();
const I18N_KEYS_SERIES: &[&str] = &["nav_series", "content_series"].as_slice();

/// GET /home/overview — broadcast + return SSE response for @get.
pub async fn get_home_overview(
    State(state): State<SharedState>,
    req: Request,
) -> crate::utils::response::Response {
    let ctx = extract_context(&req);
    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS_OVERVIEW);
    let patches = Shell::empty()
        .content(elements::HOME_CONTENT_OVERVIEW)
        .into_events();
    let mut events = vec![ctx.event_emitter.emit_signal_volatile(signals::ACTIVE_PAGE_OVERVIEW)];
    events.push(ctx.event_emitter.emit_signal(&i18n_signals));
    events.extend(ctx.event_emitter.emit_elements(&patches));
    sse_response(&events)
}

/// GET /home/movies — broadcast + return SSE response for @get.
pub async fn get_home_movies(
    State(state): State<SharedState>,
    req: Request,
) -> crate::utils::response::Response {
    let ctx = extract_context(&req);
    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS_MOVIES);
    let patches = Shell::empty()
        .content(elements::HOME_CONTENT_MOVIES)
        .into_events();
    let mut events = vec![ctx.event_emitter.emit_signal_volatile(signals::ACTIVE_PAGE_MOVIES)];
    events.push(ctx.event_emitter.emit_signal(&i18n_signals));
    events.extend(ctx.event_emitter.emit_elements(&patches));
    sse_response(&events)
}

/// GET /home/series — broadcast + return SSE response for @get.
pub async fn get_home_series(
    State(state): State<SharedState>,
    req: Request,
) -> crate::utils::response::Response {
    let ctx = extract_context(&req);
    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS_SERIES);
    let patches = Shell::empty()
        .content(elements::HOME_CONTENT_SERIES)
        .into_events();
    let mut events = vec![ctx.event_emitter.emit_signal_volatile(signals::ACTIVE_PAGE_SERIES)];
    events.push(ctx.event_emitter.emit_signal(&i18n_signals));
    events.extend(ctx.event_emitter.emit_elements(&patches));
    sse_response(&events)
}
