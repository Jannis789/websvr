// platform-core — Pure domain types. NO I/O, NO Rama dependency.
pub mod client_id;
pub mod config;
pub mod session;
pub mod client_context;
pub mod event_emitter;
pub mod buffered_event;
pub mod sse_broadcaster;
pub mod i18n;
pub mod password;

// Re-exports
pub use client_id::ClientId;
pub use config::Config;
pub use session::{SessionStorage, StorageMode};
pub use client_context::ClientContext;
pub use event_emitter::EventEmitter;
pub use buffered_event::BufferedEvent;
pub use sse_broadcaster::SseBroadcaster;
pub use i18n::{I18n, Lang};
pub use password::{PasswordUtil, compute_content_hash};
