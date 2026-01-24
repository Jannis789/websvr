use async_stream::stream;
use rama::futures::Stream;
use rama::http::sse::Event;
use rama::http::sse::datastar::EventData;
use rama::http::sse::datastar::{ElementPatchMode, PatchElements};
use rama::utils::str::NonEmptyStr;
use std::convert::Infallible;
use tokio::time::{Duration, sleep};
// ================= PatchConfig =================
#[derive(Debug, Clone, Default)]
pub struct PatchConfig {
    pub mode: ElementPatchMode,
    pub selector: Option<String>,
    pub content: Option<String>,
}

// ================= Patcher =================
pub struct Patcher {
    patches: Vec<PatchConfig>,
}

impl Patcher {
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
        }
    }

    // Flow: set mehrere Patches auf einmal
    pub fn set(mut self, patches: Vec<PatchConfig>) -> Self {
        for cfg in &patches {
            self.validate(cfg);
        }
        self.patches.extend(patches);
        self
    }

    fn validate(&self, cfg: &PatchConfig) {
        match cfg.mode {
            ElementPatchMode::Replace => {
                if cfg.content.is_none() {
                    panic!("❌ content required for Replace mode");
                }
            }
            ElementPatchMode::Remove => {
                if cfg.selector.is_none() {
                    panic!("❌ selector required for Remove mode");
                }
                if cfg.content.is_some() {
                    panic!("❌ content not allowed in Remove mode");
                }
            }
            _ => {
                if cfg.selector.is_none() {
                    panic!("❌ selector required for mode {:?}", cfg.mode);
                }
                if cfg.content.is_none() {
                    panic!("❌ content required for mode {:?}", cfg.mode);
                }
            }
        }
    }

    // Build liefert den Stream
    pub fn build(self) -> impl Stream<Item = Result<Event<EventData>, Infallible>> {
        stream! {
            for cfg in self.patches {
                let event = match cfg.mode {
                    ElementPatchMode::Remove => {
                        PatchElements::new_remove(NonEmptyStr::try_from(cfg.selector.unwrap()).unwrap())
                    }
                    ElementPatchMode::Replace => {
                        // with_mode unnecessary, Replace is default
                        PatchElements::new(NonEmptyStr::try_from(cfg.content.unwrap()).unwrap())
                    }
                    _ => {
                        let mut patch = PatchElements::new(NonEmptyStr::try_from(cfg.content.unwrap()).unwrap())
                            .with_selector(NonEmptyStr::try_from(cfg.selector.unwrap()).unwrap());
                        patch = patch.with_mode(cfg.mode);
                        patch
                    }
                };

                sleep(Duration::from_millis(250)).await; // delay, remove for production
                yield Ok(EventData::from(event)
                    .try_into_sse_event()
                      .expect("Invalid DataStar event"));
            }
        }
    }
}

// ================= Makro für JS-ähnliche Patch-Literals =================
#[macro_export]
macro_rules! patch {
    ({ $($field:ident : $value:expr),+ $(,)? }) => {
        PatchConfig {
            $(
                $field: patch!(@map $field $value),
            )*
            ..Default::default()
        }
    };
    (@map selector $value:expr) => { Some($value.into()) };
    (@map content $value:expr) => { Some($value.into()) };
    (@map mode $value:expr) => { $value };
}
