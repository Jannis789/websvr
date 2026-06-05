use crate::components::sidebar::Sidebar;
use crate::components::Shell;
use crate::context::SharedState;
use crate::elog;
use crate::ui::constants::elements;
use crate::utils::request::extract_context;
use crate::utils::response::{html_response, Response};
use rama::http::service::web::extract::State;
use rama::http::Request;

const I18N_KEYS: &[&str] = [
    "app_name",
    "login_title",
    "login_subtitle",
    "login_email",
    "login_password",
    "login_submit",
    "login_create_account",
]
.as_slice();

/// GET /login — login page
pub async fn login_page(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    elog!(Debug, "Handler → login_page (client_id={})", ctx.client_id);

    // Cache leeren: nur Events DIESER Page im Cache (keine Altlasten)
    ctx.event_emitter.clear();

    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS);
    let patches = Shell::empty()
        .sidebar(Sidebar::clear())
        .header(elements::AUTH_HEADER)
        .content(elements::AUTH_LOGIN_FORM)
        .into_events();

    ctx.event_emitter.emit_signal(&i18n_signals);
    ctx.event_emitter.emit_elements(&patches);

    html_response(elements::SHELL)
}
