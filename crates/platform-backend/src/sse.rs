//! SSE utilities — re-exports core types for convenience.
//!
//! The actual SSE endpoint lives in [`crate::handlers::sse_handler`].

pub use platform_core::{BufferedEvent, EventEmitter, SseBroadcaster};
