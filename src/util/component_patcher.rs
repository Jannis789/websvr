use async_stream::stream;
use rama::futures::Stream;
use rama::http::sse::datastar::{ElementPatchMode, PatchElements, EventData};
use std::convert::Infallible;
use crate::util::component::Component;

// ================= ComponentPatchConfig =================

#[derive(Default)]
pub struct ComponentPatchConfig {
    pub mode: ElementPatchMode,
    pub selector: Option<String>,
    pub content: Option<Box<dyn Component + Send + Sync>>,
}

// ================= ComponentPatcher =================

pub struct ComponentPatcher {
    patches: Vec<ComponentPatchConfig>,
}

impl ComponentPatcher {
    pub fn new() -> Self {
        Self {
            patches: Vec::new(),
        }
    }

    pub fn set(mut self, patches: Vec<ComponentPatchConfig>) -> Self {
        for cfg in &patches {
            self.validate(cfg);
        }
        self.patches = patches;
        self
    }

    fn validate(&self, cfg: &ComponentPatchConfig) {
        match cfg.mode {
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
                    panic!("❌ content (Component) required for mode {:?}", cfg.mode);
                }
            }
        }
    }

    pub fn build(self) -> impl Stream<Item = Result<rama::http::sse::Event<EventData>, Infallible>> {
        for cfg in &self.patches {
            if let Some(comp) = &cfg.content {
                comp.on_before_all_patched();
            }
        }

        stream! {
            for cfg in &self.patches {
                if let Some(comp) = &cfg.content {
                    comp.on_before_patch();
                }

                let event_res = match cfg.mode.clone() {
                    ElementPatchMode::Remove => {
                        cfg.selector
                            .as_ref()
                            .map(|sel| {
                                PatchElements::new_remove(sel.as_str().try_into().unwrap())
                            })
                            .ok_or("selector missing for Remove mode")
                    }
                    _ => {
                        let rendered = if let Some(comp) = &cfg.content {
                            comp.render(None)
                        } else {
                            panic!("content unexpectedly missing");
                        };

                        if rendered.trim().is_empty() {
                            panic!("rendered content is empty for mode {:?}", cfg.mode);
                        }

                        let mut patch = PatchElements::new(rendered.as_str().try_into().unwrap());
                        if let Some(selector) = &cfg.selector {
                            patch = patch.with_selector(selector.as_str().try_into().unwrap());
                        }
                        patch = patch.with_mode(cfg.mode.clone());
                        Ok(patch)
                    }
                };

                match event_res {
                    Ok(event) => {
                        #[cfg(debug_assertions)]
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

                        yield Ok(EventData::from(event).try_into_sse_event().unwrap());

                        if let Some(comp) = &cfg.content {
                            comp.on_next_patch();
                        }
                    }
                    Err(e) => {
                        eprintln!("Patch construction failed: {}", e);
                    }
                }
            }

            for cfg in &self.patches {
                if let Some(comp) = &cfg.content {
                    comp.on_all_patched();
                }
            }
        }
    }
}

// ================= Makro (verwendet nun ComponentPatcher-Typen) =================

#[macro_export]
macro_rules! component_patch {
    ({ $($field:ident : $value:expr),+ $(,)? }) => {
        $crate::util::component_patcher::ComponentPatchConfig {
            $(
                $field: component_patch!(@map $field $value),
            )*
            ..Default::default()
        }
    };

    // Content ist jetzt ein Component → in Box wrappen
    (@map content $value:expr) => {
        Some(Box::new($value) as Box<dyn $crate::util::component::Component + Send + Sync>)
    };

    // Selector bleibt String
    (@map selector $value:expr) => { Some($value.into()) };

    // Mode wird direkt übernommen
    (@map mode $value:expr) => { $value };

    // Ignoriere alte Closure-Felder (optional, für Rückwärtskompatibilität)
    (@map before_patch $value:expr) => { None };
    (@map after_patch $value:expr) => { None };
    (@map failed_patch $value:expr) => { None };
}