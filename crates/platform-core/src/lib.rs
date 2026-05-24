// platform-core — Pure domain types. NO async runtime, NO SSE, NO HTTP.
pub mod client_id;
pub mod config;
pub mod session;
pub mod i18n;
pub mod password;

// Re-exports
pub use client_id::ClientId;
pub use config::Config;
pub use session::{SessionStorage, StorageMode};
pub use i18n::{I18n, Lang};
pub use password::PasswordUtil;
