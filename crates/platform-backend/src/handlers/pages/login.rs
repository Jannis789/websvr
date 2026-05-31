use crate::components::Shell;
use crate::context::SharedState;
use crate::elog;
use crate::utils::request::extract_context;
use crate::utils::response::{html_response, Response};
use rama::http::service::web::extract::State;
use rama::http::Request;

static SHELL: &str = include_str!("../../../assets/fragments/shell.html");
static AUTH_HEADER: &str = include_str!("../../../assets/fragments/auth/header.html");
static LOGIN_FORM: &str = include_str!("../../../assets/fragments/auth/login-form.html");

const I18N_KEYS: &[&str] = &[
    "app_name", "login_title", "login_subtitle",
    "login_email", "login_password", "login_submit",
    "login_create_account",
].as_slice();

/// GET /login — login page
pub async fn login_page(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    elog!(Debug, "Handler → login_page (client_id={})", ctx.client_id);

    let i18n_signals = state.i18n.resolve_signals(ctx.lang, I18N_KEYS);

    Shell::empty()
        .header(AUTH_HEADER)
        .content(LOGIN_FORM)
        .signals(&i18n_signals)
        .emit(&ctx);

    html_response(SHELL)
}
