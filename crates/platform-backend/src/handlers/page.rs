use rama::http::{Request, Response};
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common::{self, html_response};

/// GET /login — login page
pub async fn login_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    tracing::debug!("Handler → login_page (public route)");
    html_response(include_str!("../../assets/templates/login.html"))
}

/// GET /register — registration page
pub async fn register_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    tracing::debug!("Handler → register_page (public route)");
    html_response(include_str!("../../assets/templates/register.html"))
}

/// GET /home — main application shell
pub async fn home_page(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = common::extract_context(&req);
    tracing::debug!("Handler → home_page (client_id={})", _ctx.client_id);
    html_response(include_str!("../../assets/templates/home.html"))
}
