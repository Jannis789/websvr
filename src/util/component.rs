use serde_json::Value;
use std::collections::HashMap;
use rama::http::service::web::WebService;
use rama::http::service::web::response::IntoResponse;

/// Trait für Komponenten
pub trait Component {
    #[allow(dead_code)]
    // -----------------------
    // 1. Optional: bindet die Komponente an einen WebService
    // -----------------------
    /// `serve` kann eigene Routen (GET/POST/SSE) beim WebService registrieren.
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

    #[allow(dead_code)]
    // -----------------------
    // 4. Optional: liefert dynamische Patch-Streams inkl. Nested Components
    // -----------------------
    /// `handle_patch` liefert SSE-Patches, optional. 
    /// Wenn die Komponente Unterkomponenten hat, werden deren Patches automatisch eingebunden.
    fn handle_patch(&self) -> impl IntoResponse where Self: Sized {
        // Default: nichts
    }
    
    #[allow(dead_code)]
    /// Hook vor einzelnen Patches
    fn on_before_patch(&self) {

    }

    #[allow(dead_code)]
    /// Hook vor allen Patches
    fn on_before_all_patched(&self) {

    }

    #[allow(dead_code)]
    /// Hook direkt nach einzelnen Patches
    fn on_next_patch(&self) {

    }

    #[allow(dead_code)]
    /// Hook nach allen Patches
    fn on_all_patched(&self) {

    }
}
