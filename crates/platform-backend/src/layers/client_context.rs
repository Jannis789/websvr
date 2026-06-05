use crate::elog;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use crate::context::ClientContext;
use crate::sse::EventEmitter;
use platform_core::session::SessionStorage;
use platform_core::{ClientId, Config, Lang};
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::header;
use rama::http::response::Response;
use rama::http::Request;
use rama::service::Service;
use std::convert::Infallible;
use tokio::sync::Mutex as AsyncMutex;

/// Buffer time after SSE disconnect — gelesen aus Config::sse_disconnect_buffer_secs.
/// Default: 30s, konfigurierbar via `SSE_DISCONNECT_BUFFER_SECS`.

/// Per-client state — session, emitter + Lifecycle-Tracking.
#[derive(Debug)]
struct ClientState {
    session: Arc<AsyncMutex<SessionStorage>>,
    emitter: EventEmitter,
    last_active: Instant,
    sse_gen: AtomicU64,
}

/// Handle für SSE-Lifecycle-Cleanup. Wird dem ClientContext mitgegeben.
#[derive(Clone, Debug)]
pub struct SseCleanupHandle {
    client_id: ClientId,
    sse_gen: Arc<AtomicU64>,
    clients: Weak<Mutex<HashMap<ClientId, ClientState>>>,
}

/// Drop-Guard: Wenn der SSE-Stream stirbt, wird nach Buffer-Zeit aufgeräumt.
pub struct SseCleanupGuard {
    handle: SseCleanupHandle,
    gen: u64,
}

impl Drop for SseCleanupGuard {
    fn drop(&mut self) {
        let handle = self.handle.clone();
        let gen = self.gen;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(
                Config::global().sse_disconnect_buffer_secs,
            ))
            .await;
            if let Some(clients) = handle.clients.upgrade() {
                if let Ok(mut map) = clients.lock() {
                    if let Some(state) = map.get(&handle.client_id) {
                        if state.sse_gen.load(Ordering::SeqCst) == gen {
                            map.remove(&handle.client_id);
                            elog!(
                                Debug,
                                "SseCleanup → removed stale client {}",
                                handle.client_id
                            );
                        } else {
                            elog!(
                                Debug,
                                "SseCleanup → client {} reconnected, keeping",
                                handle.client_id
                            );
                        }
                    }
                }
            }
        });
    }
}

impl SseCleanupHandle {
    pub fn guard(&self) -> SseCleanupGuard {
        let gen = self.sse_gen.fetch_add(1, Ordering::SeqCst) + 1;
        SseCleanupGuard {
            handle: self.clone(),
            gen,
        }
    }
}

/// Single service managing per-client state (session + emitter) in ONE HashMap.
///
/// Kein globaler SseBroadcaster mehr — Events gehen per mpsc direkt zum Client.
#[derive(Debug, Clone)]
pub struct ClientContextService<S> {
    inner: S,
    clients: Arc<Mutex<HashMap<ClientId, ClientState>>>,
    server_epoch: u64,
}

impl<S> ClientContextService<S> {
    pub fn new(inner: S, server_epoch: u64) -> Self {
        Self {
            inner,
            clients: Arc::new(Mutex::new(HashMap::new())),
            server_epoch,
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

    async fn serve(&self, req: Request) -> Result<Self::Output, Self::Error> {
        let client_id = req
            .extensions()
            .get::<ClientId>()
            .copied()
            .expect("ClientId must be injected by ValidateRequestHeaderLayer");

        let had_cookie = req
            .extensions()
            .get::<CookieWasPresent>()
            .map(|c| c.0)
            .unwrap_or(false);

        let _is_sse = req.uri().path() == "/sse";

        // Ein HashMap-Lookup — session + emitter + cleanup handle
        let (session, emitter, cleanup_handle) = {
            let mut clients = match self.clients.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let state = clients.entry(client_id).or_insert_with(|| {
                elog!(Debug, "ClientState → new for {}", client_id);
                ClientState {
                    session: Arc::new(AsyncMutex::new(SessionStorage::new(client_id))),
                    emitter: EventEmitter::new(self.server_epoch),
                    last_active: Instant::now(),
                    sse_gen: AtomicU64::new(0),
                }
            });
            state.last_active = Instant::now();
            let handle = SseCleanupHandle {
                client_id,
                sse_gen: Arc::new(AtomicU64::new(state.sse_gen.load(Ordering::SeqCst))),
                clients: Arc::downgrade(&self.clients),
            };
            (state.session.clone(), state.emitter.clone(), handle)
        };

        // ClientContext bauen
        let mut ctx = ClientContext::with_session(client_id, session);
        ctx.event_emitter = emitter;

        // Sprache
        let lang = Lang::from_header(
            req.headers()
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok()),
        );
        ctx = ctx.with_lang(lang);

        // Kein begin_request/disconnect mehr nötig — Events warten im
        // Puffer bis /sse connected und drained sie direkt in den Channel.

        ctx.cleanup_handle = Some(cleanup_handle);

        let mut req = req;
        req.extensions_mut().insert(ctx);
        elog!(Debug, "ClientContext → assembled for client_id={}", client_id);

        let mut response = self.inner.serve(req).await.unwrap();

        // Set-Cookie für neue Clients
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

/// Marker extension injected by the ValidateRequestHeader closure.
#[derive(Debug, Clone)]
pub struct CookieWasPresent(pub bool);
