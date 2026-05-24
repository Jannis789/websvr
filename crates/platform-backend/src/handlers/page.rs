use crate::elog;
use rama::http::Request;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common::{self, html_response};
use crate::components::Shell;
use crate::components::sidebar::Sidebar;

static SHELL: &str = include_str!("../../assets/fragments/shell.html");

static SIDEBAR_HEADER: &str = include_str!("../../assets/fragments/sidebar/header.html");
static SIDEBAR_MENU: &str = include_str!("../../assets/fragments/sidebar/menu.html");
static SIDEBAR_FOOTER: &str = include_str!("../../assets/fragments/sidebar/footer.html");
static MAIN_HEADER: &str = include_str!("../../assets/fragments/main/header.html");
static CONTENT_OVERVIEW: &str = include_str!("../../assets/fragments/content/overview.html");

/// GET /login — login page
pub async fn login_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> common::Response {
    elog!(Debug, "Handler → login_page (public route)");
    html_response(include_str!("../../assets/templates/login.html"))
}

/// GET /register — registration page
pub async fn register_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> common::Response {
    elog!(Debug, "Handler → register_page (public route)");
    html_response(include_str!("../../assets/templates/register.html"))
}

/// GET /home — main application shell, pushes all components via SSE
pub async fn home_page(
    State(_state): State<SharedState>,
    req: Request,
) -> common::Response {
    let ctx = common::extract_context(&req);
    elog!(Debug, "Handler → home_page (client_id={})", ctx.client_id);

    Shell::empty()
        .sidebar(
            Sidebar::empty()
                .header(SIDEBAR_HEADER)
                .menu(SIDEBAR_MENU)
                .footer(SIDEBAR_FOOTER)
        )
        .header(MAIN_HEADER)
        .content_cached(CONTENT_OVERVIEW)
        .emit(&ctx);

    html_response(SHELL)
}
