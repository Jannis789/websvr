use rama::http::body::sse::datastar::PatchElements;

/// A single SSE patch: HTML data + Datastar patch config + cache flag.
pub struct PatchEntry {
    pub data: &'static str,
    pub elements: PatchElements,
    pub should_cache: bool,
}

/// Strategy interface for composable components.
/// Anything implementing `Patch` can be added to a `Shell`.
pub trait Patch {
    fn into_patches(self) -> Vec<PatchEntry>;
}
