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
}

impl Patch for Sidebar {
    fn into_patches(self) -> Vec<PatchEntry> {
        self.patches
    }
}
