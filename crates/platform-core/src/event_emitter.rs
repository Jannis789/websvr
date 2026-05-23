use std::sync::{Arc, Mutex};

use crate::BufferedEvent;

/// Simple per-client event buffer.
///
/// Stores `BufferedEvent`s so they can be replayed when an SSE
/// client reconnects.  No namespace logic, no hash-tracking —
/// that intelligence lives in the `SseEndpoint`.
///
/// Uses `Arc<Mutex<Vec<...>>>` internally so that all clones of
/// a `ClientContext` share the same buffer.  This is critical for
/// Phase 1 (replay) of the SSE endpoint: handlers emit events into
/// a clone of the same `EventEmitter` that the SSE endpoint reads from.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    buffer: Arc<Mutex<Vec<BufferedEvent>>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self { buffer: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Append an event to the buffer.
    pub fn buffer_event(&self, event: BufferedEvent) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.push(event);
        }
    }

    /// Return a snapshot of all buffered events (does NOT drain).
    /// Returns a cloned `Vec` to avoid holding the lock across async boundaries.
    pub fn get_buffered_events(&self) -> Vec<BufferedEvent> {
        self.buffer.lock().map(|buf| buf.clone()).unwrap_or_default()
    }

    /// Drain and return all buffered events, clearing the buffer.
    pub fn drain_all_events(&self) -> Vec<BufferedEvent> {
        self.buffer.lock().map(|mut buf| std::mem::take(&mut *buf)).unwrap_or_default()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
