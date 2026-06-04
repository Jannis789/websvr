use crate::components::{Fragment, Patch, PatchEntry};

pub struct Sidebar {
    patches: Vec<PatchEntry>,
}

impl Sidebar {
    pub fn full(html: &'static str) -> Self {
        Sidebar {
            patches: Fragment::new("#sidebar-slot", html).into_patches(),
        }
    }

    /// Clear the sidebar slot (empty content).
    /// Uses an HTML comment instead of empty string to avoid ByteStr EmptyStrErr.
    pub fn clear() -> Self {
        Sidebar {
            patches: Fragment::new("#sidebar-slot", "<!-- -->").into_patches(),
        }
    }
}

impl Patch for Sidebar {
    fn into_patches(self) -> Vec<PatchEntry> {
        self.patches
    }
}
