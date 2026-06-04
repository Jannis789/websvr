use super::BufferedEvent;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// A multi-producer, multi-consumer SSE broadcaster.
///
/// Wraps a `tokio::sync::broadcast` channel so that every connected
/// SSE client receives events pushed by handlers.
///
/// Holds a random `epoch` generated once at server startup.
/// Used to detect server restarts — the SW invalidates its cache
/// when the epoch changes between reconnects.
///
/// Lives in `platform-backend` alongside `EventEmitter` and `BufferedEvent`.
/// Shared via `Arc<SseBroadcaster>` in `SharedState` and `ClientContext`.
#[derive(Debug)]
pub struct SseBroadcaster {
    sender: broadcast::Sender<BufferedEvent>,
    /// Unique epoch generated once at creation. Changes on server restart.
    epoch: u64,
}

impl SseBroadcaster {
    /// Create a new broadcaster with the given channel capacity.
    /// Generates a unique epoch from the current timestamp.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        let epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self { sender, epoch }
    }

    /// Subscribe to the broadcast channel.
    /// Returns a receiver that gets all future events.
    pub fn subscribe(&self) -> broadcast::Receiver<BufferedEvent> {
        self.sender.subscribe()
    }

    /// Broadcast an event to all subscribers.
    /// Returns the number of subscribers that received the event.
    pub fn broadcast(
        &self,
        event: BufferedEvent,
    ) -> Result<usize, broadcast::error::SendError<BufferedEvent>> {
        self.sender.send(event)
    }

    /// The epoch for this server instance. Sent as `X-SSE-Epoch` header.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rama::http::body::sse::datastar::EventData;

    #[test]
    fn test_broadcast_and_receive() {
        let broadcaster = SseBroadcaster::new(16);
        let mut rx = broadcaster.subscribe();
        let event = BufferedEvent::new(
            EventData::PatchSignals(rama::http::body::sse::datastar::PatchSignals::new(
                r#"{"x":1}"#.to_string(),
            )),
            1,
        );

        let sent = broadcaster.broadcast(event).unwrap();
        assert!(sent > 0);

        let received = rx.try_recv().expect("should receive event");
        assert_eq!(received.ver(), 1);
    }

    #[test]
    fn test_epoch_is_set() {
        let broadcaster = SseBroadcaster::new(16);
        assert!(broadcaster.epoch() > 0);
    }
}
