// platform-core — Pure domain types. NO async runtime, NO SSE, NO HTTP.
pub mod client_id;
pub mod config;
pub mod i18n;
pub mod password;
pub mod session;

// Re-exports
pub use client_id::ClientId;
pub use config::Config;
pub use i18n::{I18n, Lang};
pub use password::PasswordUtil;
pub use session::{SessionStorage, StorageMode};
