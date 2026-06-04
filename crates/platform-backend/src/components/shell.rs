use super::{sidebar, Fragment, Patch};
use rama::http::body::sse::datastar::EventData;

/// Builds a list of PatchElement events from composable components.
/// Does NOT emit — the handler passes `into_events()` to `event_emitter.emit_elements()`.
pub struct Shell {
    pub(crate) events: Vec<EventData>,
}

impl Shell {
    pub fn empty() -> Self {
        Shell { events: vec![] }
    }

    /// Add any component that implements Patch.
    pub fn add(mut self, component: impl Patch) -> Self {
        for entry in component.into_patches() {
            self.events.push(entry.data);
        }
        self
    }

    /// Convenience: header-slot.
    pub fn header(self, html: &'static str) -> Self {
        self.add(Fragment::new("#header-slot", html))
    }

    /// Convenience: content-slot.
    pub fn content(self, html: &'static str) -> Self {
        self.add(Fragment::new("#content-slot", html))
    }

    /// Convenience: sidebar component.
    pub fn sidebar(self, sidebar: sidebar::Sidebar) -> Self {
        self.add(sidebar)
    }

    /// Consume the shell and return the collected PatchElement events.
    pub fn into_events(self) -> Vec<EventData> {
        self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Fragment;

    #[test]
    fn test_shell_empty() {
        assert!(Shell::empty().into_events().is_empty());
    }

    #[test]
    fn test_shell_add_fragment() {
        let frag = Fragment::new("#slot", "<span>x</span>");
        let shell = Shell::empty().add(frag);
        assert_eq!(shell.into_events().len(), 1);
    }

    #[test]
    fn test_shell_header_creates_event() {
        let shell = Shell::empty().header("<div>test</div>");
        assert_eq!(shell.into_events().len(), 1);
    }

    #[test]
    fn test_shell_chain() {
        let events = Shell::empty()
            .header("<header />")
            .content("<main />")
            .into_events();
        assert_eq!(events.len(), 2);
    }
}
