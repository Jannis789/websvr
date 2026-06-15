use std::io::Write;
use std::sync::Arc;

use rama::http::body::sse::datastar::EventData;
use rama::http::sse::EventDataWrite;

/// A single SSE event.
///
/// `is_dedup`: wenn true, serialisiert zu `id:{ver}\n\n` (kein data).
/// Der SW erkennt daran, dass es ein Cache-Replay ist.
#[derive(Debug, Clone)]
pub struct BufferedEvent {
    ver: u64,
    is_dedup: bool,
    pub(crate) data: Arc<EventData>,
}

impl BufferedEvent {
    pub fn new(data: EventData, patch_ver: u64) -> Self {
        Self {
            ver: patch_ver,
            is_dedup: false,
            data: Arc::new(data),
        }
    }

    /// Dedup-Event: selbe Version, aber nur `id: N\n\n` senden.
    pub fn new_dedup(original_ver: u64, data: EventData) -> Self {
        Self {
            ver: original_ver,
            is_dedup: true,
            data: Arc::new(data),
        }
    }

    pub fn ver(&self) -> u64 {
        self.ver
    }

    pub fn is_dedup(&self) -> bool {
        self.is_dedup
    }

    pub fn content_eq(&self, other: &EventData) -> bool {
        self.data.as_ref() == other
    }

    /// SSE-Rohformat:
    /// - Dedup:  `id:{ver}\n\n`
    /// - Voll:   `id:{ver}\nevent:{type}\ndata: {line}\n...\n\n`
    pub fn to_sse_raw_string(&self) -> String {
        if self.is_dedup {
            return format!("id:{}\n\n", self.ver);
        }

        let event_type = match self.data.as_ref() {
            EventData::PatchSignals(_) => "datastar-patch-signals",
            EventData::PatchElements(_) => "datastar-patch-elements",
            EventData::ExecuteScript(_) => "datastar-patch-elements",
        };

        let mut raw = Vec::new();
        let _ = write!(&mut raw, "id:{}\nevent:{}\n", self.ver, event_type);
        {
            let mut data_buf = Vec::new();
            let _ = self.data.as_ref().write_data(&mut data_buf);
            for line in String::from_utf8_lossy(&data_buf).lines() {
                if !line.trim().is_empty() {
                    let _ = write!(&mut raw, "data: {}\n", line);
                }
            }
        }
        raw.push(b'\n');
        String::from_utf8(raw).unwrap_or_else(|_| format!("id:{}\n\n", self.ver))
    }

    /// Legacy — für Sse-Helper.
    pub fn to_sse_event_with_id(
        &self,
    ) -> Result<rama::http::sse::Event<EventData>, rama::http::sse::EventBuildError> {
        self.data
            .as_ref()
            .clone()
            .try_into_sse_event()
            .and_then(|e| e.try_with_id(self.ver.to_string()))
    }
}
