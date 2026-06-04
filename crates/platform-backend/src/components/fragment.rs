use super::{Patch, PatchEntry};
use rama::http::body::sse::datastar::{ElementPatchMode, EventData, PatchElements};

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
            data: EventData::PatchElements(
                PatchElements::new(self.html.try_into().unwrap())
                    .with_selector(self.selector.try_into().unwrap())
                    .with_mode(ElementPatchMode::Inner),
            ),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Shell;

    #[test]
    fn test_fragment_creates_patch_entry() {
        let frag = Fragment::new("#test-slot", "<div>hello</div>");
        let patches = frag.into_patches();
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_shell_empty() {
        let shell = Shell::empty();
        assert!(shell.events.is_empty());
    }

    #[test]
    fn test_shell_add_fragment() {
        let frag = Fragment::new("#slot", "<span>x</span>");
        let shell = Shell::empty().add(frag);
        assert_eq!(shell.events.len(), 1);
    }
}
