use crate::elog;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use rama::service::Service;
use rama::http::Request;
use rama::http::header;
use rama::http::response::Response;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use platform_core::{ClientId, SessionStorage, Config};
use crate::client_context::ClientContext;
use crate::sse::{SseBroadcaster, EventEmitter};
use std::convert::Infallible;
use std::future::Future;

/// Service that assembles the full `ClientContext` from:
///   - `ClientId` (injected by `ValidateRequestHeaderLayer::custom_fn`)
///   - `SessionStorage` (injected by `SessionStorageService`)
///   - Per-client `EventEmitter` (reused across requests)
///   - `Arc<SseBroadcaster>` (shared from `SharedState`)
///
/// Also handles `Set-Cookie` for new clients — if the `platform_cid`
/// cookie was not present on the request, the response gets one.
///
/// Wrapped as a layer via `rama::layer::layer_fn`.
#[derive(Debug, Clone)]
pub struct ClientContextService<S> {
    inner: S,
    sse_broadcaster: Arc<SseBroadcaster>,
    emitters: Arc<Mutex<HashMap<ClientId, EventEmitter>>>,
}

impl<S> ClientContextService<S> {
    pub fn new(inner: S, sse_broadcaster: Arc<SseBroadcaster>) -> Self {
        Self {
            inner,
            sse_broadcaster,
            emitters: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<S, ResBody> Service<Request> for ClientContextService<S>
where
    S: Service<Request, Output = Response<ResBody>, Error = Infallible>,
    ResBody: Default + From<String> + Send + 'static,
{
    type Output = Response<ResBody>;
    type Error = Infallible;

    fn serve(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send + '_ {
        async move {
            let client_id = req.extensions()
                .get::<ClientId>()
                .copied()
                .expect("ClientId must be injected by ValidateRequestHeaderLayer");

            let session = req.extensions()
                .get::<SessionStorage>()
                .cloned()
                .expect("SessionStorage must be injected by SessionStorageService");

            // Check if cookie was already present (set by ValidateRequestLayer)
            let had_cookie = req.extensions()
                .get::<CookieWasPresent>()
                .map(|c| c.0)
                .unwrap_or(true);

            // Reuse or create a per-client EventEmitter
            let event_emitter = {
                let mut emitters = match self.emitters.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                emitters.entry(client_id).or_insert_with(EventEmitter::new).clone()
            };

            let ctx = ClientContext::with_session_and_emitter(
                client_id,
                session,
                self.sse_broadcaster.clone(),
                event_emitter,
            );

            let mut req = req;
            req.extensions_mut().insert(ctx);
            elog!(Debug, "ClientContext → assembled for client_id={}", client_id);

            let mut response = self.inner.serve(req).await.unwrap();

            // Set cookie only for new clients (no existing cookie)
            if !had_cookie {
                let ttl_days = Config::global().client_id_ttl_days;
                let max_age = ttl_days as u64 * 24 * 60 * 60;
                elog!(Debug, "ClientContext → new cookie for {} (TTL={}d)", client_id, ttl_days);
                let cookie = format!(
                    "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                    platform_core::client_id::CLIENT_ID_COOKIE,
                    client_id,
                    max_age,
                );
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    cookie.parse().unwrap(),
                );
            }

            Ok(response)
        }
    }
}

/// Marker extension injected by the ValidateRequest closure to signal
/// whether the cookie was already present (so we don't re-set it).
#[derive(Debug, Clone)]
pub struct CookieWasPresent(pub bool);
