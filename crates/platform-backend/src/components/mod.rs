use rama::http::body::sse::datastar::PatchElements;
use platform_core::ClientContext;
use crate::context::ClientContextSseExt;

pub mod sidebar;

// ── PatchEntry: data + elements for a single SSE patch ──

pub struct PatchEntry {
    pub data: &'static str,
    pub elements: PatchElements,
}

// ── Patch trait: strategy interface for composable components ──

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
    pub fn sidebar(self, sidebar: sidebar::Sidebar) -> Self {
        self.add(sidebar)
    }

    /// Emit all collected patches via SSE.
    pub fn emit(self, ctx: &ClientContext) {
        for entry in self.patches {
            ctx.emit_patch(entry.data, entry.elements, true);
        }
    }
}
