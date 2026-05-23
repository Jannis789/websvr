use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use rama::service::Service;
use rama::http::Request;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::response::Response;
use platform_core::{ClientId, SessionStorage, ClientContext, SseBroadcaster, EventEmitter};
use std::convert::Infallible;
use std::future::Future;

/// Service that aggregates `ClientId` and `SessionStorage` into a
/// `ClientContext` and injects it into `req.extensions()`.
///
/// Also attaches the `Arc<SseBroadcaster>` (from `SharedState`) so
/// handlers can emit SSE events directly from the context.
///
/// Maintains a per-client `EventEmitter` map so that all requests
/// from the same client share the same event buffer.  This is critical
/// for Phase 1 (replay) of the SSE endpoint.
///
/// **Memory note:** The `emitters` HashMap grows unboundedly with unique
/// ClientId entries.  TODO: Add TTL-based eviction for entries not accessed
/// in > 30 minutes (e.g., via `retain()` on every Nth request).
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
                .expect("ClientId must be injected by AuthService");

            let session = req.extensions()
                .get::<SessionStorage>()
                .cloned()
                .expect("SessionStorage must be injected by SessionStorageService");

            // Reuse or create a per-client EventEmitter so buffered events
            // survive across requests.  This is essential for Phase 1 replay.
            let event_emitter = {
                let mut emitters = self.emitters.lock().expect("emitters lock poisoned");
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
            crate::elog!(Ok, "ClientContext → assembled for client_id={}", client_id);

            Ok(self.inner.serve(req).await.unwrap())
        }
    }
}
