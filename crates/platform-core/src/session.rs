use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::ClientId;

/// Storage mode per session key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMode {
    /// Emitted once on the SSE stream, never persisted.
    FireAndForget,
    /// Held in RAM for the session lifetime; lost on server restart.
    Volatile,
    /// Persisted to the database via SeaORM.
    Persistent,
}

/// JSON-based per-client session state.
///
/// Keys are set with a [`StorageMode`] that controls
/// whether the value is persisted to the database,
/// kept only in RAM, or emitted once.
#[derive(Debug, Clone)]
pub struct SessionStorage {
    pub client_id: ClientId,
    data: JsonValue,
    modes: HashMap<String, StorageMode>,
}

impl SessionStorage {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            data: JsonValue::Object(Default::default()),
            modes: HashMap::new(),
        }
    }

    /// Set a value at the given JSON path with the specified storage mode.
    pub fn set(&mut self, path: &str, value: JsonValue, mode: StorageMode) {
        let path = path.trim_start_matches('.');
        if let JsonValue::Object(ref mut map) = self.data {
            map.insert(path.to_string(), value);
        }
        self.modes.insert(path.to_string(), mode);
    }

    /// Convenience: set and fire-once.
    pub fn set_fire_and_forget(&mut self, path: &str, value: JsonValue) {
        self.set(path, value, StorageMode::FireAndForget);
    }

    /// Convenience: set as volatile (RAM only).
    pub fn set_volatile(&mut self, path: &str, value: JsonValue) {
        self.set(path, value, StorageMode::Volatile);
    }

    /// Convenience: set as persistent (DB-backed).
    pub fn set_persistent(&mut self, path: &str, value: JsonValue) {
        self.set(path, value, StorageMode::Persistent);
    }

    /// Read a value by its path.
    pub fn get(&self, path: &str) -> Option<&JsonValue> {
        let path = path.trim_start_matches('.');
        if let JsonValue::Object(ref map) = self.data {
            map.get(path)
        } else {
            None
        }
    }

    /// Emit the value at a specific path (for SSE PatchSignals).
    pub fn emit_path(&self, path: &str) -> Option<JsonValue> {
        let path = path.trim_start_matches('.');
        self.get(path).cloned()
    }

    /// Return the entire session data as a JSON object.
    pub fn emit_all(&self) -> &JsonValue {
        &self.data
    }

    /// Return all persistent key/value pairs for DB storage.
    pub fn persistent_data(&self) -> JsonValue {
        let mut map = serde_json::Map::new();
        if let JsonValue::Object(ref data_map) = self.data {
            for (key, value) in data_map {
                if self.modes.get(key) == Some(&StorageMode::Persistent) {
                    map.insert(key.clone(), value.clone());
                }
            }
        }
        JsonValue::Object(map)
    }

    /// Rehydrate from a previously persisted JSON blob.
    pub fn from_persisted(client_id: ClientId, persisted: JsonValue) -> Self {
        let mut this = Self::new(client_id);
        if let JsonValue::Object(ref persisted_map) = persisted {
            if let JsonValue::Object(ref mut data_map) = this.data {
                for (key, value) in persisted_map {
                    data_map.insert(key.clone(), value.clone());
                    this.modes.insert(key.clone(), StorageMode::Persistent);
                }
            }
        }
        this
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn new_storage() -> SessionStorage {
        SessionStorage::new(ClientId::generate())
    }

    #[test]
    fn test_set_and_get() {
        let mut s = new_storage();
        s.set_volatile("name", json!("Alice"));
        assert_eq!(s.get("name").and_then(|v| v.as_str()), Some("Alice"));
    }

    #[test]
    fn test_get_nonexistent() {
        let s = new_storage();
        assert_eq!(s.get("missing"), None);
    }

    #[test]
    fn test_persistent_data_filters() {
        let mut s = new_storage();
        s.set_volatile("session", json!("temp"));
        s.set_persistent("user_id", json!(42));
        let persisted = s.persistent_data();
        assert!(persisted.get("user_id").is_some());
        assert!(persisted.get("session").is_none());
    }

    #[test]
    fn test_from_persisted_roundtrip() {
        let mut original = new_storage();
        let cid = original.client_id;
        original.set_persistent("theme", json!("dark"));
        let persisted = original.persistent_data();

        let restored = SessionStorage::from_persisted(cid, persisted);
        assert_eq!(restored.get("theme").and_then(|v| v.as_str()), Some("dark"));
        // Restored keys have Persistent mode
        let restored_persisted = restored.persistent_data();
        assert!(restored_persisted.get("theme").is_some());
    }

    #[test]
    fn test_fire_and_forget_not_in_persistent() {
        let mut s = new_storage();
        s.set_fire_and_forget("flash", json!("hello"));
        let persisted = s.persistent_data();
        assert!(persisted.get("flash").is_none());
    }
}
