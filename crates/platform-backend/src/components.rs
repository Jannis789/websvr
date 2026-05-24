use rama::http::body::sse::datastar::PatchElements;
use platform_core::ClientContext;
use crate::context::ClientContextSseExt;

/// A single patch: the raw HTML (for hashing) and the PatchElements (for SSE).
pub struct PatchEntry {
    pub data: &'static str,
    pub elements: PatchElements,
}

/// Strategy trait: anything that can produce SSE patches.
pub trait Patch {
    fn into_patches(self) -> Vec<PatchEntry>;
}

// ── Fragment: one slot, one HTML ──

pub struct Fragment {
    selector: &'static str,
    html: &'static str,
}

impl Fragment {
    pub fn new(selector: &'static str, html: &'static str) -> Self {
        Fragment { selector, html }
    }
}

impl Patch for Fragment {
    fn into_patches(self) -> Vec<PatchEntry> {
        vec![PatchEntry {
            data: self.html,
            elements: PatchElements::new(self.html.try_into().unwrap())
                .with_selector(self.selector.try_into().unwrap()),
        }]
    }
}

// ── Sidebar: multiple slots ──

pub struct Sidebar {
    patches: Vec<PatchEntry>,
}

impl Sidebar {
    pub fn empty() -> Self {
        Sidebar { patches: vec![] }
    }

    pub fn header(mut self, html: &'static str) -> Self {
        self.patches.extend(Fragment::new("#sidebar-header", html).into_patches());
        self
    }

    pub fn menu(mut self, html: &'static str) -> Self {
        self.patches.extend(Fragment::new("#sidebar-menu", html).into_patches());
        self
    }

    pub fn footer(mut self, html: &'static str) -> Self {
        self.patches.extend(Fragment::new("#sidebar-footer", html).into_patches());
        self
    }
}

impl Patch for Sidebar {
    fn into_patches(self) -> Vec<PatchEntry> {
        self.patches
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

    /// Convenience: main-header slot.
    pub fn header(self, html: &'static str) -> Self {
        self.add(Fragment::new("#main-header", html))
    }

    /// Convenience: content-body slot.
    pub fn content(self, html: &'static str) -> Self {
        self.add(Fragment::new("#content-body", html))
    }

    /// Convenience: sidebar component.
    pub fn sidebar(self, sidebar: Sidebar) -> Self {
        self.add(sidebar)
    }

    /// Emit all collected patches via SSE and return the shell HTML.
    pub fn emit(self, ctx: &ClientContext) {
        for entry in self.patches {
            ctx.emit_patch(entry.data, entry.elements, true);
        }
    }
}
