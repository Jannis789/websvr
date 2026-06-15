use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::BufferedEvent;
use crate::elog;
use rama::http::body::sse::datastar::{EventData, ExecuteScript, PatchElements, PatchSignals};
use rama::utils::str::NonEmptyStr;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Per-client event emitter.
///
/// Speichert NUR den letzten State pro Signal-Key / Slot-Selector.
/// Kein FIFO-Cache, keine History. Beim Reconnect wird nur der
/// aktuelle Snapshot als full-Events geliefert (kein id_only).
///
/// emit_* = state setzen + live broadcast
/// POST-Handler: emit + 200 OK (kein sse_response)
/// GET-Handler: emit + sse_response(events)
#[derive(Debug, Clone)]
pub struct EventEmitter {
    /// Letzter Stand pro Key (Signal) / Selector (PatchElements).
    state: Arc<Mutex<Vec<StateEntry>>>,
    senders: Arc<Mutex<Vec<UnboundedSender<BufferedEvent>>>>,
    next_ver: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
struct StateEntry {
    key: Option<String>,
    data: EventData,
    event: BufferedEvent,
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(Vec::new())),
            senders: Arc::new(Mutex::new(Vec::new())),
            next_ver: Arc::new(AtomicU64::new(1)),
        }
    }

    fn next_ver(&self) -> u64 {
        self.next_ver.fetch_add(1, Ordering::SeqCst)
    }

    fn recover_lock<T>(result: Result<T, std::sync::PoisonError<T>>) -> T {
        match result {
            Ok(guard) => guard,
            Err(e) => {
                elog!(Warn, "EventEmitter → recovered poisoned mutex");
                e.into_inner()
            }
        }
    }

    /// Signal-Keys aus JSON extrahieren.
    fn signal_keys(json: &str) -> Vec<&str> {
        json.trim()
            .strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .map(|inner| {
                inner
                    .split(',')
                    .filter_map(|pair| {
                        let mut parts = pair.splitn(2, ':');
                        let key = parts.next()?.trim().strip_prefix('"')?.strip_suffix('"')?;
                        if key.starts_with('$') || key.is_empty() {
                            None
                        } else {
                            Some(key)
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    /// State setzen: alten Eintrag mit gleichem Key/Selector ersetzen.
    fn set_state(&self, state_key: Option<String>, data: EventData) -> BufferedEvent {
        let mut state = Self::recover_lock(self.state.lock());

        // Dedup: gleicher Key + gleicher Content → id-only (is_dedup=true)
        if let Some(ref key) = state_key {
            if let Some(pos) = state.iter().position(|e| e.key.as_deref() == Some(key.as_str()) && e.data == data) {
                let original_ver = state[pos].event.ver();
                let dedup = BufferedEvent::new_dedup(original_ver, EventData::clone(&data));
                elog!(
                    Info,
                    "dedup content match key={} → id-only ver={}",
                    key,
                    original_ver,
                );
                return dedup;
            }
        }

        let ver = self.next_ver();
        let event = BufferedEvent::new(data.clone(), ver);
        let entry = StateEntry {
            key: state_key,
            data,
            event: event.clone(),
        };

        // Alten Eintrag mit gleichem Key ersetzen
        if let Some(ref k) = entry.key {
            if let Some(pos) = state.iter().position(|e| e.key.as_deref() == Some(k.as_str())) {
                elog!(Debug, "state replace key={} (old_ver={} → new_ver={})", k, state[pos].event.ver(), ver);
                state[pos] = entry;
                return event;
            }
        }

        elog!(Debug, "state push ver={} (state_len={})", ver, state.len());
        state.push(entry);
        event
    }

    // ── emit_*: state + live broadcast ──

    pub fn emit_signal(&self, signals_json: &str) -> BufferedEvent {
        let keys = Self::signal_keys(signals_json);
        let data = EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
        let state_key = keys.first().map(|s| (*s).to_string());
        let event = self.set_state(state_key, data);
        self.broadcast(&event);
        event
    }

    pub fn emit_element(&self, patch: PatchElements) -> BufferedEvent {
        let selector = patch.selector.clone().map(|s| s.to_string());
        let data = EventData::PatchElements(patch);
        let event = self.set_state(selector, data);
        self.broadcast(&event);
        event
    }

    pub fn emit_signals(&self, signals: &[&str]) -> Vec<BufferedEvent> {
        signals.iter().map(|s| self.emit_signal(s)).collect()
    }

    pub fn emit_elements(&self, elements: &[EventData]) -> Vec<BufferedEvent> {
        elements
            .iter()
            .filter_map(|e| {
                if let EventData::PatchElements(p) = e {
                    Some(self.emit_element(p.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Gecachtes ExecuteScript (wird bei Replay wiederholt).
    pub fn try_emit_script(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let exec = ExecuteScript::new(non_empty);
        let event = self.set_state(None, EventData::ExecuteScript(exec));
        self.broadcast(&event);
        Some(event)
    }

    // ── Volatile: live broadcast + return (NIEMALS state) ──

    pub fn emit_signal_volatile(&self, signals_json: &str) -> BufferedEvent {
        let ver = self.next_ver();
        let data = EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
        let event = BufferedEvent::new(data, ver);
        self.broadcast(&event);
        event
    }

    pub fn try_emit_script_volatile(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let ver = self.next_ver();
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), ver);
        self.broadcast(&event);
        Some(event)
    }

    // ── Broadcast: live send an alle SSE-Streams ──

    pub fn broadcast(&self, event: &BufferedEvent) -> bool {
        let mut senders = Self::recover_lock(self.senders.lock());
        let before = senders.len();
        let mut delivered = 0usize;
        senders.retain(|tx| {
            if tx.send(event.clone()).is_ok() {
                delivered += 1;
                true
            } else {
                false
            }
        });
        let stale = before - senders.len();
        if delivered > 0 || stale > 0 {
            elog!(
                Info,
                "broadcast ver={} → {} live, {} stale",
                event.ver(),
                delivered,
                stale,
            );
        }
        delivered > 0
    }

    // ── Connect: Sender registrieren + live-Rx zurück ──
    //
    // client_seen: Anzahl Events im SW-FIFO.
    // - Match (client_seen == cached_count) → State als id-only pushen (SW replays)
    // - Mismatch → State als volle Events pushen
    pub fn connect(&self, client_seen: usize) -> UnboundedReceiver<BufferedEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let state_len = self.cached_count();
        let mismatch = client_seen != state_len;

        {
            let state = Self::recover_lock(self.state.lock());
            for entry in state.iter() {
                if mismatch {
                    // Volles Event
                    let _ = tx.send(entry.event.clone());
                } else {
                    // id-only (SW hat es schon, soll nur ID zum Replay kriegen)
                    let dedup = BufferedEvent::new_dedup(entry.event.ver(), EventData::clone(&entry.data));
                    let _ = tx.send(dedup);
                }
            }
        }

        let senders_count;
        {
            let mut senders = Self::recover_lock(self.senders.lock());
            senders.push(tx);
            senders_count = senders.len();
        }

        elog!(
            Info,
            "connect → {} senders (pushed {} events, mismatch={})",
            senders_count,
            state_len,
            mismatch,
        );

        rx
    }

    pub fn cached_count(&self) -> usize {
        Self::recover_lock(self.state.lock()).len()
    }

    pub fn current_ver(&self) -> u64 {
        self.next_ver.load(Ordering::SeqCst)
    }
}
