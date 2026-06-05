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
/// `clear()` leert den Cache NICHT — es markiert nur den Start-Index
/// der aktuellen Page. `connect()` liefert nur Events AB diesem Index.
///
/// **Dedup**: Sucht im GESAMTEN Cache (auch alte Pages) → selber Content
/// bekommt dieselbe Version — selbst über Page-Reloads hinweg.
///
/// **Fan-out**: Live-Events an ALLE verbundenen mpsc-Sender.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    cache: Arc<Mutex<Vec<BufferedEvent>>>,
    page_start: Arc<AtomicUsize>,
    senders: Arc<Mutex<Vec<UnboundedSender<BufferedEvent>>>>,
    next_ver: Arc<AtomicU64>,
    epoch: u64,
}

const MAX_CACHE: usize = 200;

impl EventEmitter {
    pub fn new(epoch: u64) -> Self {
        Self {
            cache: Arc::new(Mutex::new(Vec::new())),
            page_start: Arc::new(AtomicUsize::new(0)),
            senders: Arc::new(Mutex::new(Vec::new())),
            next_ver: Arc::new(AtomicU64::new(1)),
            epoch,
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

    /// Dedup im GESAMTEN Cache (auch alte Pages):
    /// Selber Content → selbe Version.
    fn dedup(&self, candidate: &EventData) -> Option<BufferedEvent> {
        let cache = Self::recover_lock(self.cache.lock());
        for entry in cache.iter() {
            if entry.content_eq(candidate) {
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
            // Atomar page_start anpassen, damit es nicht auf gelöschte Einträge zeigt
            self.page_start.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ps| {
                Some(ps.saturating_sub(1))
            }).ok();
        }
    }

    fn send_live(&self, event: BufferedEvent) {
        let mut senders = Self::recover_lock(self.senders.lock());
        senders.retain(|tx| tx.send(event.clone()).is_ok());
    }

    fn send_and_cache(&self, event: BufferedEvent) {
        self.send_live(event.clone());
        self.push_cache(event);
    }

    /// Cached: Live-Fan-out + Cache. Dedup findet Event beim nächsten Mal.
    pub fn emit_element(&self, patch: PatchElements) -> BufferedEvent {
        let candidate_data = EventData::PatchElements(patch.clone());
        if let Some(existing) = self.dedup(&candidate_data) {
            self.send_live(existing.clone());
            self.push_cache(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(EventData::PatchElements(patch), ver);
        self.send_and_cache(event.clone());
        event
    }

    /// Volatile: NUR Live-Fan-out, KEIN Cache. Für transienten Content
    /// wie $activePage-Signale, die bei Reconnect nicht relevant sind.
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
        let candidate_data = EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
        if let Some(existing) = self.dedup(&candidate_data) {
            self.send_live(existing.clone());
            self.push_cache(existing.clone());
            return existing;
        }
        let ver = self.next_ver();
        let event = BufferedEvent::new(candidate_data, ver);
        self.send_and_cache(event.clone());
        event
    }

    /// Volatile: NUR Live-Fan-out, KEIN Cache. Für Signale die sich
    /// ständig ändern (activePage, Timestamps, Session-Daten).
    pub fn emit_signal_volatile(&self, signals_json: &str) -> BufferedEvent {
        let candidate_data = EventData::PatchSignals(PatchSignals::new(signals_json.to_string()));
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
    /// Für One-Shot-Scripts wie Redirects oder Transient-Toasts.
    pub fn try_emit_script_volatile(&self, script: &str) -> Option<BufferedEvent> {
        let non_empty = NonEmptyStr::try_from(script).ok()?;
        let ver = self.next_ver();
        let exec = ExecuteScript::new(non_empty);
        let event = BufferedEvent::new(EventData::ExecuteScript(exec), ver);
        self.send_live(event.clone());
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
    /// **Page-Scoping**: `clear()` merkt sich den Start-Index der
    /// aktuellen Page. `connect()` liefert NUR Events ab diesem
    /// Index — keine Events von vorherigen Pages.
    ///
    /// **Dedup**: Der Cache enthält auch alte Events (für Dedup),
    /// aber `connect()` filtert sie per Index aus.
    pub fn connect(
        &self,
        client_ver: u64,
        client_epoch: u64,
    ) -> (UnboundedReceiver<BufferedEvent>, ReplayPlan) {
        let (tx, rx) = mpsc::unbounded_channel();
        {
            let mut senders = Self::recover_lock(self.senders.lock());
            senders.push(tx);
        }

        let cache = Self::recover_lock(self.cache.lock());
        let start = self.page_start.load(Ordering::SeqCst);
        let epoch_match = client_epoch == 0 || client_epoch == self.epoch;

        let mut id_only = Vec::new();
        let mut full = Vec::new();

        for (i, event) in cache.iter().enumerate() {
            // Nur Events der aktuellen Page (ab page_start)
            if i < start {
                continue;
            }

            if !epoch_match || event.ver() > client_ver {
                full.push(event.clone());
            } else {
                id_only.push(event.ver());
            }
        }

        // Dedupliziere id_only und full — bei Dedup-Hits kann dieselbe Version
        // mehrfach im Cache auftauchen (Original + Push vom Dedup-Hit).
        id_only.sort_unstable();
        id_only.dedup();
        full.sort_unstable_by_key(|e| e.ver());
        full.dedup_by_key(|e| e.ver());

        if !id_only.is_empty() || !full.is_empty() {
            elog!(
                Debug,
                "SSE → replay plan (start={}): {} id_only, {} full (client_ver={}, epoch_match={})",
                start,
                id_only.len(),
                full.len(),
                client_ver,
                epoch_match
            );
        }

        (rx, ReplayPlan { id_only, full })
    }

    /// Markiert den Start einer neuen Page.
    /// NICHT den Cache leeren — alte Events bleiben für Dedup erhalten.
    /// Nur der Replay-Plan wird auf aktuelle Page gescoped.
    pub fn clear(&self) {
        let cache = Self::recover_lock(self.cache.lock());
        let prev = self.page_start.swap(cache.len(), Ordering::SeqCst);
        elog!(
            Debug,
            "EventEmitter → page start {} (was {}, cache has {} total)",
            cache.len(),
            prev,
            cache.len()
        );
    }

    pub fn cached_count(&self) -> usize {
        Self::recover_lock(self.cache.lock()).len()
    }

    pub fn current_ver(&self) -> u64 {
        self.next_ver.load(Ordering::SeqCst)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
