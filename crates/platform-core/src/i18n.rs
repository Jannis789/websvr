use serde::{Serialize, Deserialize};
use serde_json::Value as JsonValue;

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    De,
    En,
}

impl Lang {
    /// Detect language from the `Accept-Language` header.
    /// Defaults to English.
    pub fn from_header(header: Option<&str>) -> Self {
        match header {
            Some(h) if h.starts_with("de") => Lang::De,
            _ => Lang::En,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Lang::De => "de",
            Lang::En => "en",
        }
    }
}

/// Immutable i18n translation map.
///
/// Loaded once at startup from `assets/i18n/{de,en}.json`.
/// Supports DB overrides in a later phase.
#[derive(Debug, Clone)]
pub struct I18n {
    de: JsonValue,
    en: JsonValue,
}

impl I18n {
    pub fn new(de_json: JsonValue, en_json: JsonValue) -> Self {
        Self {
            de: de_json,
            en: en_json,
        }
    }

    /// Get the full translation map for a language.
    pub fn get(&self, lang: Lang) -> &JsonValue {
        match lang {
            Lang::De => &self.de,
            Lang::En => &self.en,
        }
    }

    /// Resolve a translation key to its string value.
    /// Returns `None` if the key doesn't exist.
    pub fn resolve(&self, lang: Lang, key: &str) -> Option<String> {
        let map = self.get(lang);
        map.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}
