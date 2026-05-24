use std::sync::{Arc, Mutex};

use super::BufferedEvent;

/// Per-client event buffer keyed by selector.
///
/// Stores the **latest** `BufferedEvent` per CSS selector so that SSE
/// replay always sends the current state, not a history of patches.
/// Navigation events (should_cache: false) never enter the buffer,
/// which guarantees the initial state is always correct on reload.
///
/// Uses `Arc<Mutex<HashMap>>` internally so that all clones of
/// a `ClientContext` share the same buffer.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    buffer: Arc<Mutex<Vec<BufferedEvent>>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self { buffer: Arc::new(Mutex::new(Vec::new())) }
    }

    /// Append an event to the buffer, replacing any existing event
    /// with the same CSS selector. This keeps the buffer small and
    /// ensures replay always reflects the current state.
    pub fn buffer_event(&self, event: BufferedEvent) {
        if let Ok(mut buf) = self.buffer.lock() {
            // Extract selector from payload to deduplicate
            let selector = event.extract_selector();
            if let Some(sel) = selector {
                buf.retain(|e| e.extract_selector().as_ref() != Some(&sel));
            }
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
