use super::{Patch, PatchEntry};
use rama::http::body::sse::datastar::{ElementPatchMode, PatchElements};

/// One slot, one HTML fragment.
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
            elements: PatchElements::new(self.html.try_into().unwrap())
                .with_selector(self.selector.try_into().unwrap())
                .with_mode(ElementPatchMode::Inner),
        }]
    }
}
