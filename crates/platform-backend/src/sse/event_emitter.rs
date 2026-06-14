use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
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
/// **Cache**: Persistiert über Pages hinweg (FIFO-eviction bei MAX_CACHE).
/// **page_start**: Wird vom Layer via `next_page()` auf `cache.len()` gesetzt.
/// `connect()` replays NUR Events ab `page_start` — und verändert `page_start` NICHT.
/// So werden Events vorheriger Pages nie replayt, aber Fast-Reload verliert keine Events.
/// **page_gen**: Wird vom Layer pro non-/sse Request inkrementiert.
/// Via `x-sse-gen` Header an den SW übermittelt (nicht-destruktiv).
///
/// **Dedup**: Sucht im GESAMTEN Cache (auch alte Pages).
/// **Fan-out**: `send_live()` feuert an ALLE live Sender.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    cache: Arc<Mutex<Vec<BufferedEvent>>>,
    page_start: Arc<AtomicUsize>,
    page_gen: Arc<AtomicU64>,
    senders: Arc<Mutex<Vec<UnboundedSender<BufferedEvent>>>>,
    next_ver: Arc<AtomicU64>,
}

const MAX_CACHE: usize = 200;

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(Vec::new())),
            page_start: Arc::new(AtomicUsize::new(0)),
            page_gen: Arc::new(AtomicU64::new(1)),
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

    /// Page-Generation — vom Layer pro non-/sse Request aufgerufen.
    /// Setzt `page_start` auf 0, sodass der neue Connect ALLE Events
    /// aus dem Cache bekommt (auch die von vorherigen Pages).
    /// Wird via `x-sse-gen` Header an den SW übermittelt.
    pub fn next_page(&self) -> u64 {
        let new_gen = self.page_gen.fetch_add(1, Ordering::SeqCst) + 1;
        // page_start auf cache.len() setzen = alle bisherigen Events werden
        // beim nächsten Replay übersprungen. Nur Events die NACH der Navigation
        // in den Cache kommen (via emit_element/emit_signal) landen nach diesem
        // Index und werden beim nächsten SSE-Connect replayt.
        let cache_len = Self::recover_lock(self.cache.lock()).len();
        self.page_start.store(cache_len, Ordering::SeqCst);
        new_gen
    }

    /// Aktuelle Page-Generation für den SSE-Header.
    pub fn current_gen(&self) -> u64 {
        self.page_gen.load(Ordering::SeqCst)
    }

    /// Dedup im GESAMTEN Cache (auch alte Pages):
    /// Selber Content → selbe Version.
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
            // page_start mitverschieben, damit es nicht ins Leere zeigt
            self.page_start
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v.saturating_sub(1)))
                .ok();
        }
    }

    /// Live-Fan-out ohne Cache-Fallback — fuer volatile Events wie ExecuteScript.
    /// Dead sender werden aufgeraeumt. Wenn kein live Sender, wird das Event
    /// STILLSCHWEIGEND verworfen (kein Cache).
    fn send_live_only(&self, event: BufferedEvent) {
        let mut senders = Self::recover_lock(self.senders.lock());
        let sender_count = senders.len();
        senders.retain(|tx| tx.send(event.clone()).is_ok());
        if sender_count > 0 {
            elog!(Debug, "send_live_only ver={}: tried {} senders", event.ver(), sender_count);
        }
    }

    /// Live-Fan-out an ALLE verbundenen Sender.
    /// Dead sender (rx dropped) werden aufgeräumt.
    /// Returns `true` wenn das Event in den Cache gefallen ist (kein live Sender).
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

    /// Live-Fan-out + Cache (ohne Doppel-Push).
    fn send_and_cache(&self, event: BufferedEvent) {
        if !self.send_live(event.clone()) {
            self.push_cache(event);
        }
    }

    /// Cached: Live-Fan-out + Cache. Dedup findet Event beim nächsten Mal.
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

    /// Volatile: NUR Live-Fan-out, KEIN Cache.
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

    /// Cached: Live-Fan-out + Cache. Dedup findet Signal beim nächsten Mal.
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

    /// Volatile: NUR Live-Fan-out, KEIN Cache.
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

    /// Cached: Script wird gecached und bei Reconnect erneut ausgeführt.
    pub fn try_emit_script(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let ver = self.next_ver();
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), ver);
        self.send_and_cache(event.clone());
        Some(event)
    }

    /// Volatile: Script wird NUR live ausgeführt, nie gecached.
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

    /// Connect: registriert Sender (Fan-out) + baut Replay-Plan.
    ///
    /// **Stale-Erkennung**: Wenn `client_gen > 0` und `client_gen != current_gen()`,
    /// ist dies eine alte Page die reconnectet → Sender registrieren, aber LEERES Replay.
    /// Nur die aktuelle Page kriegt Events — keine Doppel-Delivery.
    ///
    /// Filtert NUR Events ab `page_start` (gesetzt vom Layer via `next_page()`).
    /// `page_start` wird hier NICHT verändert — nur der Layer setzt es.
    /// Innerhalb des Fensters [page_cache_len, cache.len()]: `ver`-basiert.
    pub fn connect(
        &self,
        client_ver: u64,
        client_gen: u64,
    ) -> (UnboundedReceiver<BufferedEvent>, ReplayPlan) {
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut senders = Self::recover_lock(self.senders.lock());
            senders.push(tx);
        }

        // Stale-Connection-Check: alter Client reconnectet mit veraltetem page_gen.
        // Vergleicht mit `current_gen()` — der Layer inkrementiert page_gen NUR für
        // Full-Page-Requests (nicht für In-Page-Nav). Daher ist ein Gen-Mismatch
        // immer ein echtes "alte Page reconnectet".
        let current = self.current_gen();
        let is_stale = client_gen > 0 && client_gen != current;

        if is_stale {
            elog!(
                Debug,
                "SSE → stale connect (client_gen={}, current_gen={}) — empty replay",
                client_gen,
                current
            );
            return (rx, ReplayPlan { id_only: vec![], full: vec![] });
        }

        let cache = Self::recover_lock(self.cache.lock());

        let start = self.page_start.load(Ordering::SeqCst);

        let mut id_only = Vec::new();
        let mut full = Vec::new();

        for (i, event) in cache.iter().enumerate() {
            if i < start {
                // Events vor page_start: id_only wenn Client sie kennt
                if event.ver() > client_ver {
                    full.push(event.clone());
                } else {
                    id_only.push(event.ver());
                }
            } else {
                // Events ab page_start (aktuelle Seite): immer full,
                // da der Client sie noch nicht im SW-Cache haben kann
                full.push(event.clone());
            }
        }


        drop(cache);

        // Dedupliziere id_only und full
        id_only.sort_unstable();
        id_only.dedup();
        full.sort_unstable_by_key(|e| e.ver());
        full.dedup_by_key(|e| e.ver());

        if !id_only.is_empty() || !full.is_empty() {
            elog!(
                Info,
                "SSE → replay plan:  {} id_only, {} full (client_ver={})",
                id_only.len(),
                full.len(),
                client_ver,
            );
        } else {
            elog!(
                Debug,
                "SSE → replay empty (client_ver={})",
                client_ver,
            );
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
