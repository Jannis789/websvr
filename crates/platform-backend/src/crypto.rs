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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let a = compute_content_hash("hello", "secret");
        let b = compute_content_hash("hello", "secret");
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_different_data() {
        let a = compute_content_hash("hello", "secret");
        let b = compute_content_hash("world", "secret");
        assert_ne!(a, b);
    }

    #[test]
    fn test_hash_different_secret() {
        let a = compute_content_hash("hello", "secret1");
        let b = compute_content_hash("hello", "secret2");
        assert_ne!(a, b);
    }

    #[test]
    fn test_hash_length_is_32_hex_chars() {
        // 16 bytes = 32 hex characters
        let hash = compute_content_hash("test", "key");
        assert_eq!(hash.len(), 32);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
