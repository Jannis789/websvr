use crate::components::Shell;
use crate::context::SharedState;
use crate::elog;
use crate::utils::request::extract_context;
use crate::utils::response::{html_response, Response};
use rama::http::service::web::extract::State;
use rama::http::Request;

static SHELL: &str = include_str!("../../../assets/fragments/shell.html");
static AUTH_HEADER: &str = include_str!("../../../assets/fragments/auth/header.html");
static REGISTER_FORM: &str = include_str!("../../../assets/fragments/auth/register-form.html");

const I18N_KEYS: &[&str] = &[
    "app_name", "register_title", "register_subtitle",
    "register_username", "register_email", "register_password",
    "register_confirm_password", "register_submit", "register_has_account",
];

/// GET /register — registration page
pub async fn register_page(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    elog!(Debug, "Handler → register_page (client_id={})", ctx.client_id);

    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS);

    Shell::empty()
        .header(AUTH_HEADER)
        .content(REGISTER_FORM)
        .signals(&i18n_signals)
        .emit(&ctx);

    html_response(SHELL)
}
