use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::{BufferedEvent, SseBroadcaster};
use crate::elog;
use rama::http::body::sse::datastar::{EventData, ExecuteScript, PatchElements, PatchSignals};
use rama::utils::str::NonEmptyStr;
use tokio::sync::broadcast::Receiver;

/// Replay plan for a reconnecting client.
pub struct ReplayPlan {
    id_only: Vec<u64>,
    full: Vec<BufferedEvent>,
}

impl ReplayPlan {
    pub fn into_parts(self) -> (Vec<u64>, Vec<BufferedEvent>) {
        (self.id_only, self.full)
    }
}

const MAX_CACHE: usize = 64;

/// Cache entry with request-generation tracking.
/// Only entries whose `request_gen` matches the current `req_counter`
/// are eligible for reconnect replay.
#[derive(Debug)]
struct CacheEntry {
    event: BufferedEvent,
    request_gen: u64,
}

/// Per-client event emitter.
///
/// - Content-dedup: same content = same patch_ver.
/// - Request-scoped replay: `begin_request()` increments a generation
///   counter. Emits stamp the entry with that generation. On reconnect,
///   only entries from the current generation are replayed.
/// - Live events are sent WITHOUT `id:` — the SW only caches via replay.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    cache: Arc<Mutex<Vec<CacheEntry>>>,
    broadcaster: Arc<SseBroadcaster>,
    next_ver: Arc<AtomicU64>,
    /// Current request generation. Incremented on each `begin_request()`.
    req_counter: Arc<AtomicU64>,
}

impl EventEmitter {
    pub fn new(broadcaster: Arc<SseBroadcaster>) -> Self {
        Self {
            cache: Arc::new(Mutex::new(Vec::new())),
            broadcaster,
            next_ver: Arc::new(AtomicU64::new(1)),
            req_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    fn next_ver(&self) -> u64 {
        self.next_ver.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn current_gen(&self) -> u64 {
        self.req_counter.load(Ordering::SeqCst)
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

    /// Increment request generation.
    /// Call at the start of each handler request so that only events
    /// emitted during THIS request are eligible for reconnect replay.
    /// The SSE endpoint does NOT call this — it inherits the last
    /// handler's generation.
    pub fn begin_request(&self) {
        let old = self.req_counter.fetch_add(1, Ordering::SeqCst);
        elog!(Debug, "EventEmitter → begin_request: gen {} → {}", old, old + 1);
    }

    fn dedup_and_stamp(&self, candidate: &EventData) -> Option<BufferedEvent> {
        let gen = self.current_gen();
        let mut cache = Self::recover_lock(self.cache.lock());
        for entry in cache.iter_mut() {
            if entry.event.content_eq(candidate) {
                entry.request_gen = gen;
                return Some(entry.event.clone());
            }
        }
        None
    }

    fn cache_insert(&self, event: BufferedEvent) {
        let gen = self.current_gen();
        let mut cache = Self::recover_lock(self.cache.lock());
        cache.push(CacheEntry {
            event,
            request_gen: gen,
        });
        while cache.len() > MAX_CACHE {
            cache.remove(0);
        }
    }

    pub fn emit_element(&self, patch: PatchElements) -> BufferedEvent {
        let candidate_data = EventData::PatchElements(patch.clone());
        if let Some(existing) = self.dedup_and_stamp(&candidate_data) {
            self.broadcast_event(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(EventData::PatchElements(patch), ver);
        self.cache_insert(event.clone());
        self.broadcast_event(event.clone());
        event
    }

    pub fn emit_signal(&self, signals_json: &str) -> BufferedEvent {
        let candidate_data = EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
        if let Some(existing) = self.dedup_and_stamp(&candidate_data) {
            self.broadcast_event(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(candidate_data, ver);
        self.cache_insert(event.clone());
        self.broadcast_event(event.clone());
        event
    }

    pub fn try_emit_script(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let ver = self.next_ver();
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), ver);
        self.broadcast_event(event.clone());
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

    fn broadcast_event(&self, event: BufferedEvent) {
        match self.broadcaster.broadcast(event) {
            Ok(0) => elog!(Debug, "EventEmitter → broadcast: no subscribers"),
            Ok(_) => {}
            Err(e) => elog!(Warn, "EventEmitter → broadcast failed: {}", e),
        }
    }

    pub fn subscribe_and_plan(
        &self,
        client_ver: u64,
        client_epoch: u64,
    ) -> (Receiver<BufferedEvent>, ReplayPlan) {
        let rx = self.broadcaster.subscribe();
        let plan = self.build_replay_plan(client_ver, client_epoch);
        (rx, plan)
    }

    fn build_replay_plan(&self, client_ver: u64, client_epoch: u64) -> ReplayPlan {
        let gen = self.current_gen();

        if client_epoch != self.epoch() {
            elog!(Debug, "SSE → epoch mismatch (client={}, server={}) → full replay (gen={})", client_epoch, self.epoch(), gen);
            let full = self.snapshot_by_gen(gen);
            return ReplayPlan {
                id_only: Vec::new(),
                full,
            };
        }

        let cache = Self::recover_lock(self.cache.lock());
        let mut id_only = Vec::new();
        let mut full = Vec::new();

        for entry in cache.iter() {
            if entry.request_gen != gen {
                elog!(Debug, "SSE → skip ver={} (gen={}, current={})", entry.event.ver(), entry.request_gen, gen);
                continue;
            }
            if entry.event.ver() <= client_ver {
                elog!(Debug, "SSE → id_only ver={} (client_ver={})", entry.event.ver(), client_ver);
                id_only.push(entry.event.ver());
            } else {
                elog!(Debug, "SSE → full ver={} (client_ver={})", entry.event.ver(), client_ver);
                full.push(entry.event.clone());
            }
        }

        elog!(Debug, "SSE → replay plan: {} id_only, {} full", id_only.len(), full.len());
        ReplayPlan { id_only, full }
    }

    fn snapshot_by_gen(&self, gen: u64) -> Vec<BufferedEvent> {
        let cache = Self::recover_lock(self.cache.lock());
        cache
            .iter()
            .filter(|e| e.request_gen == gen)
            .map(|e| e.event.clone())
            .collect()
    }

    pub fn epoch(&self) -> u64 {
        self.broadcaster.epoch()
    }

    pub fn cached_count(&self) -> usize {
        Self::recover_lock(self.cache.lock()).len()
    }

    pub fn clear(&self) {
        let mut cache = Self::recover_lock(self.cache.lock());
        let count = cache.len();
        cache.clear();
        elog!(Debug, "EventEmitter → cache cleared (removed {} events)", count);
    }

    pub fn current_ver(&self) -> u64 {
        self.next_ver.load(Ordering::SeqCst)
    }

    #[doc(hidden)]
    pub fn broadcast_raw(
        &self,
        event: BufferedEvent,
    ) -> Result<usize, tokio::sync::broadcast::error::SendError<BufferedEvent>> {
        self.broadcaster.broadcast(event)
    }
}
