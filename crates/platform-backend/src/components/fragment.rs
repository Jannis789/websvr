use rama::http::body::sse::datastar::{ElementPatchMode, PatchElements};
use super::{PatchEntry, Patch};

/// One slot, one HTML fragment.
pub struct Fragment {
    selector: &'static str,
    html: &'static str,
    should_cache: bool,
}

impl Fragment {
    pub fn new(selector: &'static str, html: &'static str) -> Self {
        Fragment { selector, html, should_cache: true }
    }

    /// Uncached fragment — used for navigation-dependent slots like #content-body.
    pub fn uncached(selector: &'static str, html: &'static str) -> Self {
        Fragment { selector, html, should_cache: false }
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
