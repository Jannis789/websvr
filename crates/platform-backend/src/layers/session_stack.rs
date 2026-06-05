use platform_core::ClientId;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::layer::validate_request::ValidateRequestHeaderLayer;
use rama::http::service::web::Router;
use rama::layer::layer_fn;
use rama::Layer;

use crate::context::SharedState;
use crate::layers::client_context::{ClientContextService, CookieWasPresent};
use crate::utils;

/// Build the layer stack:
/// ClientId extraction → ClientContextService (session + emitter in einer HashMap).
pub fn session_layer(
    server_epoch: u64,
    inner: Router<SharedState>,
) -> impl rama::Service<rama::http::Request, Output = rama::http::Response, Error = std::convert::Infallible>
{
    (
        ValidateRequestHeaderLayer::custom_fn(|mut req: rama::http::Request| async move {
            let had_cookie =
                utils::request::get_cookie_value(&req, platform_core::client_id::CLIENT_ID_COOKIE).is_some();
            let client_id = extract_or_generate_client_id(&req);
            crate::elog!(Debug, "ClientId → {} (cookie={})", client_id, had_cookie);
            req.extensions_mut().insert(client_id);
            req.extensions_mut().insert(CookieWasPresent(had_cookie));
            Ok(req)
        }),
        layer_fn(|s| ClientContextService::new(s, server_epoch)),
    )
        .layer(inner)
}

fn extract_or_generate_client_id(req: &rama::http::Request) -> ClientId {
    if let Some(cid) = req.extensions().get::<ClientId>() {
        return *cid;
    }
    if let Some(cookie_str) =
        utils::request::get_cookie_value(req, platform_core::client_id::CLIENT_ID_COOKIE)
    {
        if let Some(cid) = ClientId::parse(&cookie_str) {
            return cid;
        }
    }
    ClientId::generate()
}
