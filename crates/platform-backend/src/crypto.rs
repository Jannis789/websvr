use platform_core::Config;
use ring::hmac;

/// Return the HMAC secret from the global config as a static reference.
pub fn hmac_secret() -> &'static str {
    &Config::global().hmac_secret
}

/// Compute a deterministic content hash for SSE hash-sync.
///
/// Uses HMAC-SHA256 with the server secret, truncates to
/// 16 bytes (128 bit), returns hex-encoded string.
pub fn compute_content_hash(data: &str, secret: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, data.as_bytes());
    hex::encode(&tag.as_ref()[..16])
}
