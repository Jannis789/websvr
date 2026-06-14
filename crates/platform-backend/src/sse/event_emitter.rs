use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::BufferedEvent;
use crate::elog;
use rama::http::body::sse::datastar::{EventData, ExecuteScript, PatchElements, PatchSignals};
use rama::utils::str::NonEmptyStr;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Replay-Plan für SSE-Reconnect.
#[derive(Debug)]
pub struct ReplayPlan {
    pub id_only: Vec<u64>,
    pub full: Vec<BufferedEvent>,
}

impl ReplayPlan {
    pub fn into_parts(self) -> (Vec<u64>, Vec<BufferedEvent>) {
        (self.id_only, self.full)
    }
}

/// Per-client event emitter.
///
/// **Cache**: FIFO-Cache mit Content-Dedup. Gleicher Inhalt = gleiche Version.
/// Der Dedup (`dedup()`) durchsucht den GESAMTEN Cache, pages-übergreifend.
/// **Fan-out**: `send_live()` feuert an ALLE live Sender.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    cache: Arc<Mutex<Vec<BufferedEvent>>>,
    senders: Arc<Mutex<Vec<UnboundedSender<BufferedEvent>>>>,
    next_ver: Arc<AtomicU64>,
}

const MAX_CACHE: usize = 200;

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(Vec::new())),
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

    /// Dedub im GESAMTEN Cache: gleicher Content → gleiche Version.
    fn dedup(&self, candidate: &EventData) -> Option<BufferedEvent> {
        let cache = Self::recover_lock(self.cache.lock());
        for entry in cache.iter() {
            if entry.content_eq(candidate) {
                elog!(Info, "dedup HIT ver={}", entry.ver());
                return Some(entry.clone());
            }
        }
        None
    }

    fn push_cache(&self, event: BufferedEvent) {
        let mut cache = Self::recover_lock(self.cache.lock());
        cache.push(event);
        while cache.len() > MAX_CACHE {
            cache.remove(0);
        }
    }

    /// Live-Fan-out ohne Cache-Fallback (volatile ExecuteScript).
    fn send_live_only(&self, event: BufferedEvent) {
        let mut senders = Self::recover_lock(self.senders.lock());
        let sender_count = senders.len();
        senders.retain(|tx| tx.send(event.clone()).is_ok());
        if sender_count > 0 {
            elog!(Debug, "send_live_only ver={}: tried {} senders", event.ver(), sender_count);
        }
    }

    /// Live-Fan-out + Cache-Fallback falls kein live Sender.
    fn send_live(&self, event: BufferedEvent) -> bool {
        let mut senders = Self::recover_lock(self.senders.lock());
        let mut any_received = false;
        let sender_count = senders.len();
        senders.retain(|tx| {
            if tx.send(event.clone()).is_ok() {
                any_received = true;
                true
            } else {
                false
            }
        });
        if !any_received {
            elog!(Info, "send_live ver={}: no live sender (had {}), falling back to cache", event.ver(), sender_count);
            drop(senders);
            self.push_cache(event);
            true
        } else {
            elog!(Debug, "send_live ver={}: delivered to {} live senders", event.ver(), sender_count);
            false
        }
    }

    fn send_and_cache(&self, event: BufferedEvent) {
        if !self.send_live(event.clone()) {
            self.push_cache(event);
        }
    }

    pub fn emit_element(&self, patch: PatchElements) -> BufferedEvent {
        let candidate_data = EventData::PatchElements(patch.clone());
        if let Some(existing) = self.dedup(&candidate_data) {
            self.send_and_cache(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(EventData::PatchElements(patch), ver);
        self.send_and_cache(event.clone());
        event
    }

    pub fn emit_element_volatile(&self, patch: PatchElements) -> BufferedEvent {
        let candidate_data = EventData::PatchElements(patch.clone());
        if let Some(existing) = self.dedup(&candidate_data) {
            self.send_live(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(EventData::PatchElements(patch), ver);
        self.send_live(event.clone());
        event
    }

    pub fn emit_signal(&self, signals_json: &str) -> BufferedEvent {
        let candidate_data =
            EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
        if let Some(existing) = self.dedup(&candidate_data) {
            self.send_and_cache(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(candidate_data, ver);
        self.send_and_cache(event.clone());
        event
    }

    pub fn emit_signal_volatile(&self, signals_json: &str) -> BufferedEvent {
        let candidate_data =
            EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
        if let Some(existing) = self.dedup(&candidate_data) {
            self.send_live(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(candidate_data, ver);
        self.send_live(event.clone());
        event
    }

    pub fn try_emit_script(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let ver = self.next_ver();
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), ver);
        self.send_and_cache(event.clone());
        Some(event)
    }

    pub fn try_emit_script_volatile(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let ver = self.next_ver();
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), ver);
        self.send_live_only(event.clone());
        Some(event)
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

    /// Connect: registriert Sender + baut Replay-Plan über ALLE gecachten Events.
    /// Events mit `ver > client_ver` = full (Client hat sie noch nicht).
    /// Events mit `ver <= client_ver` = id_only (Client hat sie im SW-Cache).
    pub fn connect(
        &self,
        client_ver: u64,
        _client_gen: u64,
    ) -> (UnboundedReceiver<BufferedEvent>, ReplayPlan) {
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut senders = Self::recover_lock(self.senders.lock());
            senders.push(tx);
        }

        let cache = Self::recover_lock(self.cache.lock());

        let mut id_only = Vec::new();
        let mut full = Vec::new();

        for event in cache.iter() {
            if event.ver() > client_ver {
                full.push(event.clone());
            } else {
                id_only.push(event.ver());
            }
        }

        drop(cache);

        id_only.sort_unstable();
        id_only.dedup();
        full.sort_unstable_by_key(|e| e.ver());
        full.dedup_by_key(|e| e.ver());

        if !id_only.is_empty() || !full.is_empty() {
            elog!(
                Info,
                "SSE → replay: {} id_only, {} full (client_ver={})",
                id_only.len(),
                full.len(),
                client_ver,
            );
        } else {
            elog!(Debug, "SSE → replay empty (client_ver={})", client_ver,);
        }

        (rx, ReplayPlan { id_only, full })
    }

    pub fn cached_count(&self) -> usize {
        Self::recover_lock(self.cache.lock()).len()
    }

    pub fn current_ver(&self) -> u64 {
        self.next_ver.load(Ordering::SeqCst)
    }
}
