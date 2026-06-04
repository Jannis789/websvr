use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CLIENT_ID_COOKIE: &str = "platform_cid";

/// UUID v4 based client identifier.
/// Immutable value object — stored in cookies, injected by AuthService.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClientId(pub Uuid);

impl ClientId {
    /// Generate a fresh random ClientId (v4 UUID).
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a ClientId from its hex string representation.
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    /// Return the UUID as a 16-byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_is_unique() {
        let a = ClientId::generate();
        let b = ClientId::generate();
        assert_ne!(a, b);
    }

    #[test]
    fn test_parse_valid_uuid() {
        let id = ClientId::generate();
        let s = id.to_string();
        let parsed = ClientId::parse(&s);
        assert_eq!(parsed, Some(id));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(ClientId::parse("not-a-uuid"), None);
        assert_eq!(ClientId::parse(""), None);
    }

    #[test]
    fn test_display_is_hyphenated_hex() {
        let id = ClientId::generate();
        let s = id.to_string();
        assert_eq!(s.len(), 36); // standard UUID format
        assert_eq!(s.chars().filter(|c| *c == '-').count(), 4);
    }
}
