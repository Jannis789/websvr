use rama::http::body::sse::datastar::{ElementPatchMode, PatchElements};
use platform_core::ClientContext;
use crate::context::ClientContextSseExt;

pub mod sidebar;

// ── PatchEntry: data + elements for a single SSE patch ──

pub struct PatchEntry {
    pub data: &'static str,
    pub elements: PatchElements,
    pub should_cache: bool,
}

// ── Patch trait: strategy interface for composable components ──

pub trait Patch {
    fn into_patches(self) -> Vec<PatchEntry>;
}

// ── Fragment: one slot, one HTML ──

pub struct Fragment {
    selector: &'static str,
    html: &'static str,
    should_cache: bool,
}

impl Fragment {
    pub fn new(selector: &'static str, html: &'static str) -> Self {
        Fragment {
            selector,
            html,
            should_cache: true,
        }
    }

    /// Create a fragment that won't be cached server-side or by the SW.
    /// Used for navigation-dependent slots like #content-body.
    pub fn uncached(selector: &'static str, html: &'static str) -> Self {
        Fragment {
            selector,
            html,
            should_cache: false,
        }
    }
}

impl Patch for Fragment {
    fn into_patches(self) -> Vec<PatchEntry> {
        vec![PatchEntry {
            data: self.html,
            elements: PatchElements::new(self.html.try_into().unwrap())
                .with_selector(self.selector.try_into().unwrap())
                .with_mode(ElementPatchMode::Inner),
            should_cache: self.should_cache,
        }]
    }
}

// ── Shell: collects patches, emits on consume ──

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
    /// The SW skips content-body events; the server doesn't buffer them.
    /// On reload, the page handler always sends the correct initial content.
    pub fn content(self, html: &'static str) -> Self {
        self.add(Fragment::uncached("#content-body", html))
    }

    /// Convenience: content-body slot with server-side caching enabled.
    /// Used by the initial page load (/home) so the content is available
    /// for SSE replay on reconnect.
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
