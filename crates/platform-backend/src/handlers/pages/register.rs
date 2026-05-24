use crate::elog;
use rama::http::Request;
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use crate::common::{self, html_response};

/// GET /register — registration page
pub async fn register_page(
    State(_state): State<SharedState>,
    _req: Request,
) -> common::Response {
    elog!(Debug, "Handler → register_page (public route)");
    html_response(include_str!("../../../assets/templates/register.html"))
}
