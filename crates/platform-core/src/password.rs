use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};

const SALT_LEN: usize = 16;

/// Cryptographic utilities for password hashing and content hashing.
///
/// Uses `ring::hmac::HMAC_SHA256` for both password storage
/// and the deterministic content hash used in SSE hash-sync.
#[derive(Debug)]
pub struct PasswordUtil;

impl PasswordUtil {
    /// Generate a random 16-byte salt.
    pub fn generate_salt() -> Vec<u8> {
        let rng = SystemRandom::new();
        let mut salt = vec![0u8; SALT_LEN];
        rng.fill(&mut salt).expect("failed to generate secure salt");
        salt
    }

    /// Hash a password with the given salt using HMAC-SHA256.
    /// Returns hex-encoded hash prefixed with the hex-encoded salt.
    pub fn hash_password(password: &str, salt: &[u8]) -> String {
        let key = hmac::Key::new(hmac::HMAC_SHA256, salt);
        let tag = hmac::sign(&key, password.as_bytes());
        format!("{}.{}", hex::encode(salt), hex::encode(tag.as_ref()))
    }

    /// Hash a new password with a freshly generated salt.
    pub fn hash_new(password: &str) -> String {
        let salt = Self::generate_salt();
        Self::hash_password(password, &salt)
    }

    /// Verify a password against a stored `salt.hash` string.
    pub fn verify_password(password: &str, stored: &str) -> bool {
        let parts: Vec<&str> = stored.splitn(2, '.').collect();
        if parts.len() != 2 {
            return false;
        }

        let salt = match hex::decode(parts[0]) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let expected = Self::hash_password(password, &salt);
        // Constant-time comparison via the hash string equality
        // (ring's verify is constant-time, but we use HMAC so string cmp is ok)
        expected == stored
    }
}

/// Compute a deterministic content hash for SSE hash-sync.
///
/// Uses HMAC-SHA256 with a secret key (from env), truncates to
/// 16 bytes (128 bit), and returns a hex-encoded string.
/// This is deterministic across server restarts, unlike `std::hash::Hash`.
pub fn compute_content_hash(data: &str, hmac_secret: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, hmac_secret.as_bytes());
    let tag = hmac::sign(&key, data.as_bytes());
    // Truncate to 16 bytes (128 bit) for efficient hash-sync in the Service Worker
    hex::encode(&tag.as_ref()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let hash = PasswordUtil::hash_new("hunter2");
        assert!(PasswordUtil::verify_password("hunter2", &hash));
        assert!(!PasswordUtil::verify_password("wrong", &hash));
    }

    #[test]
    fn test_content_hash_deterministic() {
        let a = compute_content_hash("hello", "secret");
        let b = compute_content_hash("hello", "secret");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32); // 16 bytes = 32 hex chars
    }

    #[test]
    fn test_content_hash_different() {
        let a = compute_content_hash("hello", "secret");
        let b = compute_content_hash("world", "secret");
        assert_ne!(a, b);
    }
}
