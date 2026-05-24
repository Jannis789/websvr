// platform-backend — HTTP server, layer stack, handlers, SSE.
pub mod components;
pub mod server;
pub mod layers;
pub mod handlers;
pub mod sse;
pub mod db;
pub mod entities;
pub mod context;
pub mod crypto;
pub mod common;
pub mod utils;

/// Centralized logging macro. Captures file:line automatically.
/// Usage: `elog!(Info, "message {}", arg)` or `elog!(Error, "oops")`
#[macro_export]
macro_rules! elog {
    ($level:ident, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::utils::log::emit(
            $crate::utils::log::Level::$level,
            file!(),
            line!(),
            &msg,
        );
    }};
}
