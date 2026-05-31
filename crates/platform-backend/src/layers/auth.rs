use rama::extensions::ExtensionsRef;
use rama::http::layer::validate_request::ValidateRequestHeaderLayer;
use rama::http::service::web::Router;
use rama::Layer;

use crate::context::SharedState;

/// Auth gate: redirect to /login if session has no "authenticated" flag.
pub fn require_auth(
    inner: Router<SharedState>,
) -> impl rama::Service<rama::http::Request, Output = rama::http::Response, Error = std::convert::Infallible>
{
    ValidateRequestHeaderLayer::custom_fn(|req: rama::http::Request| async move {
        let authenticated = req
            .extensions()
            .get::<crate::context::ClientContext>()
            .and_then(|ctx| {
                ctx.session_storage
                    .try_lock()
                    .ok()
                    .and_then(|guard| guard.get("authenticated").and_then(|v| v.as_bool()))
            })
            .unwrap_or(false);

        if authenticated {
            Ok(req)
        } else {
            crate::elog!(Debug, "Auth → /login");
            Err(rama::http::Response::builder()
                .status(rama::http::StatusCode::SEE_OTHER)
                .header("location", "/login")
                .body(rama::http::Body::empty())
                .unwrap())
        }
    })
    .layer(inner)
}
