use crate::components::sidebar::Sidebar;
use crate::components::Shell;
use crate::context::SharedState;
use crate::elog;
use crate::ui::constants::{elements, signals};
use crate::utils::request::extract_context;
use crate::utils::response::{html_response, Response};
use rama::http::service::web::extract::State;
use rama::http::Request;

const I18N_KEYS: &[&str] = &[
    "app_name",
    "aria_menu",
    "app_brand",
    "aria_close",
    "nav_overview",
    "nav_movies",
    "nav_series",
    "content_overview",
    "settings_title",
    "settings_logout",
]
.as_slice();

/// GET /home — main application shell, pushes all components via SSE
pub async fn home_page(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    elog!(Debug, "Handler → home_page (client_id={})", ctx.client_id);

    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS);
    let patches = Shell::empty()
        .add(Sidebar::full(elements::HOME_SIDEBAR))
        .header(elements::HOME_HEADER)
        .content(elements::HOME_CONTENT_OVERVIEW)
        .into_events();

    ctx.event_emitter
        .emit_signals(&[signals::ACTIVE_PAGE_OVERVIEW, &i18n_signals].as_slice());
    ctx.event_emitter.emit_elements(&patches);

    html_response(elements::SHELL)
}
