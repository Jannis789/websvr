use crate::elog;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::context::ClientContext;
use crate::sse::{EventEmitter, SseBroadcaster};
use platform_core::session::SessionStorage;
use platform_core::{ClientId, Config, Lang};
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::header;
use rama::http::response::Response;
use rama::http::Request;
use rama::service::Service;
use std::convert::Infallible;
use std::future::Future;
use tokio::sync::Mutex as AsyncMutex;

/// Service that assembles the full `ClientContext` from:
///   - `ClientId` (injected by `ValidateRequestHeaderLayer::custom_fn`)
///   - `Arc<Mutex<SessionStorage>>` (injected by `SessionStorageService`)
///   - Per-client `EventEmitter` (reused across requests)
///   - `Arc<SseBroadcaster>` (shared from `SharedState`)
///
/// Also handles `Set-Cookie` for new clients.
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

    fn serve(&self, req: Request) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send + '_ {
        async move {
            let client_id = req
                .extensions()
                .get::<ClientId>()
                .copied()
                .expect("ClientId must be injected by ValidateRequestHeaderLayer");

            let session = req
                .extensions()
                .get::<Arc<AsyncMutex<SessionStorage>>>()
                .cloned()
                .expect("SessionStorage must be injected by SessionStorageService");

            let had_cookie = req
                .extensions()
                .get::<CookieWasPresent>()
                .map(|c| c.0)
                .unwrap_or(true);

            let mut ctx = ClientContext::with_session(client_id, session, self.sse_broadcaster.clone());

            // Detect language from Accept-Language header
            let lang = Lang::from_header(
                req.headers()
                    .get(header::ACCEPT_LANGUAGE)
                    .and_then(|v| v.to_str().ok()),
            );
            ctx = ctx.with_lang(lang);

            // Reuse the per-client EventEmitter so the buffer persists across requests
            {
                let mut emitters = match self.emitters.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                ctx.event_emitter = emitters
                    .entry(client_id)
                    .or_insert_with(|| {
                        EventEmitter::new(self.sse_broadcaster.clone(), client_id.to_string())
                    })
                    .clone();
            }

            let mut req = req;
            req.extensions_mut().insert(ctx);
            elog!(Debug, "ClientContext → assembled for client_id={}", client_id);

            let mut response = self.inner.serve(req).await.unwrap();

            // Set cookie only for new clients
            if !had_cookie {
                let ttl_days = Config::global().client_id_ttl_days;
                let max_age = ttl_days as u64 * 24 * 60 * 60;
                elog!(
                    Debug,
                    "ClientContext → new cookie for {} (TTL={}d)",
                    client_id,
                    ttl_days
                );
                let cookie = format!(
                    "{}={}; Path=/; SameSite=Lax; Max-Age={}",
                    platform_core::client_id::CLIENT_ID_COOKIE,
                    client_id,
                    max_age,
                );
                response
                    .headers_mut()
                    .insert(header::SET_COOKIE, cookie.parse().unwrap());
            }

            Ok(response)
        }
    }
}

/// Marker extension injected by the ValidateRequest closure.
#[derive(Debug, Clone)]
pub struct CookieWasPresent(pub bool);
