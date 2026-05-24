use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};

const SALT_LEN: usize = 16;

/// Password hashing utilities using HMAC-SHA256.
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
    /// Uses constant-time comparison to prevent timing attacks.
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

        let expected_hash = expected.split('.').nth(1).unwrap_or("");
        let stored_hash = parts[1];
        if expected_hash.len() != stored_hash.len() {
            return false;
        }
        let cmp = expected_hash
            .bytes()
            .zip(stored_hash.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));
        cmp == 0
    }
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
}
