use std::sync::Arc;

use crate::{ClientId, SessionStorage, EventEmitter, SseBroadcaster};

/// Aggregated per-request client state.
///
/// Injected into `req.extensions()` by `ClientContextService`
/// and extracted by handlers via `extract_context(&req)`.
///
/// Contains everything a handler needs to emit SSE events
/// and access session data.
#[derive(Debug, Clone)]
pub struct ClientContext {
    pub client_id: ClientId,
    pub session_storage: SessionStorage,
    pub event_emitter: EventEmitter,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}

impl ClientContext {
    /// Create a fresh `ClientContext` for a new client.
    pub fn new(client_id: ClientId, sse_broadcaster: Arc<SseBroadcaster>) -> Self {
        Self {
            session_storage: SessionStorage::new(client_id),
            event_emitter: EventEmitter::new(),
            sse_broadcaster,
            client_id,
        }
    }

    /// Create a `ClientContext` from an existing session.
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

    /// Create a `ClientContext` with a pre-existing `EventEmitter`.
    ///
    /// This is used by `ClientContextService` to share the same event buffer
    /// across all requests from the same client, which is essential for
    /// Phase 1 (replay) of the SSE endpoint.
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
