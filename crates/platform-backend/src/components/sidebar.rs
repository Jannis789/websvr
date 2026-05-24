use crate::components::{Fragment, Patch, PatchEntry};

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
