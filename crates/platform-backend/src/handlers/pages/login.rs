use crate::elog;
use rama::http::Request;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::utils::response::{Response, html_response};

/// GET /login — login page
pub async fn login_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    elog!(Debug, "Handler → login_page (public route)");
    html_response(include_str!("../../../assets/templates/login.html"))
}