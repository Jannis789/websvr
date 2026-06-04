use crate::components::Shell;
use crate::context::SharedState;
use crate::elog;
use crate::ui::constants::elements;
use crate::utils::request::extract_context;
use crate::utils::response::{html_response, Response};
use rama::http::service::web::extract::State;
use rama::http::Request;

const I18N_KEYS: &[&str] = &[
    "app_name",
    "register_title",
    "register_subtitle",
    "register_username",
    "register_email",
    "register_password",
    "register_confirm_password",
    "register_submit",
    "register_has_account",
];

/// GET /register — registration page
pub async fn register_page(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    elog!(Debug, "Handler → register_page (client_id={})", ctx.client_id);

    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS);
    let patches = Shell::empty()
        .header(elements::AUTH_HEADER)
        .content(elements::AUTH_REGISTER_FORM)
        .into_events();

    ctx.event_emitter.emit_signal(&i18n_signals);
    ctx.event_emitter.emit_elements(&patches);

    html_response(elements::SHELL)
}
