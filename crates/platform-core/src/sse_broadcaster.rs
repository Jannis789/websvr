use crate::BufferedEvent;
use tokio::sync::broadcast;

/// A multi-producer, multi-consumer SSE broadcaster.
///
/// Wraps a `tokio::sync::broadcast` channel so that every connected
/// SSE client receives events pushed by handlers.
///
/// Defined in `platform-core` because `ClientContext` (also in core)
/// holds an `Arc<SseBroadcaster>`.
#[derive(Debug, Clone)]
pub struct SseBroadcaster {
    sender: broadcast::Sender<BufferedEvent>,
}

impl SseBroadcaster {
    /// Create a new broadcaster with the given channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribe to the broadcast channel.
    /// Returns a receiver that gets all future events.
    pub fn subscribe(&self) -> broadcast::Receiver<BufferedEvent> {
        self.sender.subscribe()
    }

    /// Broadcast an event to all subscribers.
    /// Returns the number of subscribers that received the event.
    pub fn broadcast(&self, event: BufferedEvent) -> Result<usize, broadcast::error::SendError<BufferedEvent>> {
        self.sender.send(event)
    }
}
