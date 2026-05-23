use serde::{Serialize, Deserialize};
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
