use std::sync::Arc;
use tokio::sync::Mutex;

use platform_core::{ClientId, SessionStorage};
use crate::sse::{EventEmitter, SseBroadcaster};

/// Aggregated per-request client state.
/// `session_storage` is shared via Arc<Mutex<>> so mutations persist across requests.
#[derive(Debug, Clone)]
pub struct ClientContext {
    pub client_id: ClientId,
    pub session_storage: Arc<Mutex<SessionStorage>>,
    pub event_emitter: EventEmitter,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}

impl ClientContext {
    pub fn new(client_id: ClientId, sse_broadcaster: Arc<SseBroadcaster>) -> Self {
        Self {
            session_storage: Arc::new(Mutex::new(SessionStorage::new(client_id))),
            event_emitter: EventEmitter::new(),
            sse_broadcaster,
            client_id,
        }
    }

    pub fn with_session(
        client_id: ClientId,
        session: Arc<Mutex<SessionStorage>>,
        sse_broadcaster: Arc<SseBroadcaster>,
    ) -> Self {
        Self {
            client_id,
            session_storage: session,
            event_emitter: EventEmitter::new(),
            sse_broadcaster,
        }
    }
}
