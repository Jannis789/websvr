use rama::http::{
    service::web::{WebService, response::{Html, IntoResponse}, Sse},
    sse::datastar::ElementPatchMode,
};
use crate::util::patcher::{Patcher, patch};
use serde_json::Value;
use std::collections::HashMap;

/// Trait für Komponenten
pub trait Component {
    // -----------------------
    // 1. Optional: bindet die Komponente an einen WebService
    // -----------------------
    /// `serve` kann eigene Routen (GET/POST/SSE) beim WebService registrieren.
    /// Default: tut nichts.
    fn serve(&self, svc: WebService<()>) -> WebService<()> {
        svc
    }

    // -----------------------
    // 2. Pflicht: liefert das leere Template
    // -----------------------
    /// Liefert ein statisches, ungefülltes Template als &'static str.
    fn get_template() -> &'static str
    where
        Self: Sized;

    // -----------------------
    // 3. Pflicht: rendert die Komponente dynamisch
    // -----------------------
    /// `render` füllt das Template dynamisch. 
    /// Parameter sind ein HashMap<String, Value> → beliebige Daten
    fn render(&self, params: Option<&HashMap<String, Value>>) -> String;

    // -----------------------
    // 4. Optional: liefert dynamische Patch-Streams inkl. Nested Components
    // -----------------------
    /// `handle_patch` liefert SSE-Patches, optional. 
    /// Wenn die Komponente Unterkomponenten hat, werden deren Patches automatisch eingebunden.
    fn handle_patch(&self) -> Option<Sse> {
        None
    }
}
