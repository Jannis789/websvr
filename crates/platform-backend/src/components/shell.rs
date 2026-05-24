use crate::client_context::ClientContext;
use crate::context::ClientContextSseExt;
use super::{PatchEntry, Patch, Fragment, sidebar};

/// Collects patches from components, emits them all via SSE on consume.
pub struct Shell {
    patches: Vec<PatchEntry>,
}

impl Shell {
    pub fn empty() -> Self {
        Shell { patches: vec![] }
    }

    /// Add any component that implements Patch.
    pub fn add(mut self, component: impl Patch) -> Self {
        self.patches.extend(component.into_patches());
        self
    }

    /// Convenience: main-header slot (cached).
    pub fn header(self, html: &'static str) -> Self {
        self.add(Fragment::new("#main-header", html))
    }

    /// Convenience: content-body slot (uncached — navigation-dependent).
    pub fn content(self, html: &'static str) -> Self {
        self.add(Fragment::uncached("#content-body", html))
    }

    /// Convenience: content-body slot with server-side caching enabled.
    pub fn content_cached(self, html: &'static str) -> Self {
        self.add(Fragment::new("#content-body", html))
    }

    /// Convenience: sidebar component.
    pub fn sidebar(self, sidebar: sidebar::Sidebar) -> Self {
        self.add(sidebar)
    }

    /// Emit all collected patches via SSE.
    pub fn emit(self, ctx: &ClientContext) {
        for entry in self.patches {
            ctx.emit_patch(entry.data, entry.elements, entry.should_cache);
        }
    }
}
