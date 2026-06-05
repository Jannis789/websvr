use std::sync::Arc;
use tokio::sync::Mutex;

use platform_core::{ClientId, Lang, SessionStorage};

/// Aggregated per-request client state.
/// `session_storage` is shared via Arc<Mutex<>> so mutations persist across requests.
#[derive(Debug, Clone)]
pub struct ClientContext {
    pub client_id: ClientId,
    pub session_storage: Arc<Mutex<SessionStorage>>,
    pub event_emitter: crate::sse::EventEmitter,
    pub lang: Lang,
    /// SSE-Lifecycle-Handle — wird im ClientContextService gesetzt.
    /// Der SSE-Handler nutzt es für den delayed Cleanup bei Disconnect.
    pub cleanup_handle: Option<crate::layers::client_context::SseCleanupHandle>,
}

impl ClientContext {
    /// New context with a fresh session and a placeholder event emitter.
    /// The real per-client emitter is injected by ClientContextService.
    pub fn new(client_id: ClientId) -> Self {
        Self {
            session_storage: Arc::new(Mutex::new(SessionStorage::new(client_id))),
            event_emitter: crate::sse::EventEmitter::new(0),
            client_id,
            lang: Lang::En,
            cleanup_handle: None,
        }
    }

    /// New context with an existing session.
    /// The real per-client emitter is injected by ClientContextService.
    pub fn with_session(client_id: ClientId, session: Arc<Mutex<SessionStorage>>) -> Self {
        Self {
            client_id,
            session_storage: session,
            event_emitter: crate::sse::EventEmitter::new(0),
            lang: Lang::En,
            cleanup_handle: None,
        }
    }

    pub fn with_lang(mut self, lang: Lang) -> Self {
        self.lang = lang;
        self
    }
}
