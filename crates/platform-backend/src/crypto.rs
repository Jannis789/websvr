use platform_core::Config;

/// Return the HMAC secret from the global config as a static reference.
/// Avoids unnecessary heap allocation on every call (hot path in emit_patch).
pub fn hmac_secret() -> &'static str {
    &Config::global().hmac_secret
}
