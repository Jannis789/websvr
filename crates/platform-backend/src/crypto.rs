use ring::hmac;

/// Compute a deterministic content hash for SSE hash-sync.
///
/// Uses HMAC-SHA256 with the cookie value as key, truncates to
/// 16 bytes (128 bit), returns hex-encoded string.
/// Both server and SW compute the same hash from the same content + cookie.
pub fn compute_content_hash(data: &str, secret: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, data.as_bytes());
    hex::encode(&tag.as_ref()[..16])
}
