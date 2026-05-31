use std::sync::{Arc, Mutex};

use super::{BufferedEvent, SseBroadcaster};
use rama::http::body::sse::datastar::{EventData, PatchElements, PatchSignals, ExecuteScript};
use rama::utils::str::NonEmptyStr;

/// Per-client event emitter with slot-based state cache.
///
/// Caches the last PatchElements per selector and last PatchSignals.
/// On SSE reconnect, replays the current state — Datastar's mergePatch
/// and morph-dom make this idempotent.
///
/// The SW independently caches event payloads by their `id:` hash
/// for client-side dedup and test verification.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    /// Last PatchElements per selector (e.g. "#sidebar-slot", "#content-slot").
    slots: Arc<Mutex<Vec<BufferedEvent>>>,
    /// Last PatchSignals event.
    signals: Arc<Mutex<Option<BufferedEvent>>>,
    broadcaster: Arc<SseBroadcaster>,
    cookie_secret: String,
}

impl EventEmitter {
    pub fn new(broadcaster: Arc<SseBroadcaster>, cookie_secret: String) -> Self {
        Self {
            slots: Arc::new(Mutex::new(Vec::new())),
            signals: Arc::new(Mutex::new(None)),
            broadcaster,
            cookie_secret,
        }
    }

    /// Emit a PatchElements event. Always cached (replaces previous for same selector).
    pub fn emit_element(&self, patch: PatchElements) -> BufferedEvent {
        let event = BufferedEvent::new(EventData::PatchElements(patch), &self.cookie_secret);
        let _ = self.broadcaster.broadcast(event.clone());
        self.cache_slot(event.clone());
        event
    }

    /// Emit a PatchSignals event. Always cached.
    pub fn emit_signals(&self, signals_json: &str) -> BufferedEvent {
        let patch = PatchSignals::new(signals_json.to_string());
        let event = BufferedEvent::new(EventData::PatchSignals(patch), &self.cookie_secret);
        let _ = self.broadcaster.broadcast(event.clone());
        if let Ok(mut sig) = self.signals.lock() {
            *sig = Some(event.clone());
        }
        event
    }

    /// Emit an ExecuteScript event. Never cached (scripts are one-shot).
    pub fn emit_script(&self, script: &str) -> BufferedEvent {
        let non_empty =
            NonEmptyStr::try_from(script).expect("emit_script: script must not be empty");
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), &self.cookie_secret);
        let _ = self.broadcaster.broadcast(event.clone());
        event
    }

    /// Cache a PatchElements event, replacing any previous event for the same selector.
    fn cache_slot(&self, event: BufferedEvent) {
        if let Ok(mut slots) = self.slots.lock() {
            let selector = match &event.data {
                EventData::PatchElements(pe) => pe.selector.as_deref().map(|s| s.to_string()),
                _ => None,
            };
            if let Some(sel) = selector {
                slots.retain(|e| match &e.data {
                    EventData::PatchElements(pe) => pe.selector.as_deref() != Some(&sel),
                    _ => true,
                });
            }
            slots.push(event);
        }
    }

    /// Return the current state for replay (slots + signals).
    pub fn get_state(&self) -> Vec<BufferedEvent> {
        let mut state = Vec::new();
        if let Ok(slots) = self.slots.lock() {
            state.extend(slots.iter().cloned());
        }
        if let Ok(sig) = self.signals.lock() {
            if let Some(s) = sig.clone() {
                state.push(s);
            }
        }
        state
    }

    /// Subscribe to the broadcast channel for live events.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<BufferedEvent> {
        self.broadcaster.subscribe()
    }

    /// Broadcast a raw BufferedEvent (low-level, for tests).
    pub fn broadcast(
        &self,
        event: BufferedEvent,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<BufferedEvent>> {
        self.broadcaster.broadcast(event)
    }

    /// Number of cached slot events.
    pub fn cached_count(&self) -> usize {
        self.slots.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// The cookie secret used for hashing (needed by tests).
    pub fn secret(&self) -> &str {
        &self.cookie_secret
    }
}
