use super::{sidebar, Fragment, Patch, PatchEntry};
use crate::context::sse_response;
use crate::context::ClientContext;
use crate::utils::response::Response;

/// Collects patches from components, emits them all via SSE on consume.
pub struct Shell {
    patches: Vec<PatchEntry>,
    signals_json: Option<String>,
}

impl Shell {
    pub fn empty() -> Self {
        Shell {
            patches: vec![],
            signals_json: None,
        }
    }

    /// Add any component that implements Patch.
    pub fn add(mut self, component: impl Patch) -> Self {
        self.patches.extend(component.into_patches());
        self
    }

    /// Convenience: header-slot.
    pub fn header(self, html: &'static str) -> Self {
        self.add(Fragment::new("#header-slot", html))
    }

    /// Convenience: content-slot.
    pub fn content(self, html: &'static str) -> Self {
        self.add(Fragment::new("#content-slot", html))
    }

    /// Convenience: sidebar component.
    pub fn sidebar(self, sidebar: sidebar::Sidebar) -> Self {
        self.add(sidebar)
    }

    /// Set initial signals to emit alongside patches.
    pub fn signals(mut self, json: &str) -> Self {
        self.signals_json = Some(json.to_string());
        self
    }

    /// Broadcast all patches via SSE (for initial page load via /sse stream).
    pub fn emit(self, ctx: &ClientContext) {
        if let Some(ref json) = self.signals_json {
            ctx.event_emitter.emit_signals(json);
        }
        for entry in self.patches {
            ctx.event_emitter.emit_element(entry.elements);
        }
    }

    /// Return patches as SSE response body (for @get navigation via Datastar).
    /// Broadcasts events AND returns them as the HTTP response body.
    pub fn emit_response(self, ctx: &ClientContext) -> Response {
        let mut events = Vec::new();
        if let Some(ref json) = self.signals_json {
            let event = ctx.event_emitter.emit_signals(json);
            events.push(event);
        }
        for entry in self.patches {
            let event = ctx.event_emitter.emit_element(entry.elements);
            events.push(event);
        }
        sse_response(&events)
    }
}
