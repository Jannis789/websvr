use std::sync::Arc;

use platform_core::{ClientId, SessionStorage};
use crate::sse::{EventEmitter, SseBroadcaster};

/// Aggregated per-request client state.
#[derive(Debug, Clone)]
pub struct ClientContext {
    pub client_id: ClientId,
    pub session_storage: SessionStorage,
    pub event_emitter: EventEmitter,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}

impl ClientContext {
    pub fn new(client_id: ClientId, sse_broadcaster: Arc<SseBroadcaster>) -> Self {
        Self {
            session_storage: SessionStorage::new(client_id),
            event_emitter: EventEmitter::new(),
            sse_broadcaster,
            client_id,
        }
    }

    pub fn with_session(
        client_id: ClientId,
        session: SessionStorage,
        sse_broadcaster: Arc<SseBroadcaster>,
    ) -> Self {
        Self {
            client_id,
            session_storage: session,
            event_emitter: EventEmitter::new(),
            sse_broadcaster,
        }
    }

    pub fn with_session_and_emitter(
        client_id: ClientId,
        session: SessionStorage,
        sse_broadcaster: Arc<SseBroadcaster>,
        event_emitter: EventEmitter,
    ) -> Self {
        Self {
            client_id,
            session_storage: session,
            event_emitter,
            sse_broadcaster,
        }
    }
}
