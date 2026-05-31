use std::sync::Arc;

use platform_core::ClientId;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::layer::validate_request::ValidateRequestHeaderLayer;
use rama::http::service::web::Router;
use rama::layer::layer_fn;
use rama::Layer;

use crate::context::SharedState;
use crate::context::{new_session_map, SessionStorageService};
use crate::layers::client_context::{ClientContextService, CookieWasPresent};
use crate::sse::SseBroadcaster;
use crate::utils;

/// Create the shared session map and return the base layer stack:
/// ClientId extraction → Session storage → ClientContext assembly.
pub fn session_layer(
    broadcaster: Arc<SseBroadcaster>,
    inner: Router<SharedState>,
) -> impl rama::Service<rama::http::Request, Output = rama::http::Response, Error = std::convert::Infallible>
{
    let sessions = new_session_map();
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
        layer_fn(|s| SessionStorageService::new(s, sessions.clone())),
        layer_fn(|s| ClientContextService::new(s, broadcaster.clone())),
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
