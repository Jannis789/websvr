// src/util/patcher.rs
use async_stream::stream;
use rama::futures::Stream;
use rama::http::sse::datastar::{ElementPatchMode, PatchElements, EventData};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
// ================= PatchConfig =================
#[derive(Clone, Default)]
pub struct PatchConfig {
    pub mode: ElementPatchMode,
    pub selector: Option<String>,
    pub content: Option<String>,

    // hooks
    pub before_patch: Option<Arc<dyn Fn() + Send + Sync>>,
    pub after_patch: Option<Arc<dyn Fn() + Send + Sync>>,
    pub failed_patch: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

// ================= Patcher =================
#[derive(Clone, Default)]
pub struct Patcher {
    patches: Vec<PatchConfig>,

    // hooks
    pub before_patch: Option<Arc<dyn Fn(&PatchConfig) + Send + Sync>>,
    pub after_patch: Option<Arc<dyn Fn(&PatchConfig) + Send + Sync>>,
    pub failed_patch: Option<Arc<dyn Fn(&PatchConfig, &str) + Send + Sync>>,
    pub all_patched: Option<Arc<dyn Fn(&[PatchConfig]) + Send + Sync>>,
}
#[allow(dead_code)]
impl Patcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, patches: Vec<PatchConfig>) -> Self {
        for cfg in &patches {
            self.validate(cfg);
        }
        self.patches = patches;
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
                if cfg.selector.is_none() || cfg.content.is_none() {
                    panic!("❌ selector and content required for mode {:?}", cfg.mode);
                }
            }
        }
    }

    pub fn build(self) -> impl Stream<Item = Result<rama::http::sse::Event<EventData>, Infallible>> {
        stream! {
            for cfg in &self.patches {

                if let Some(hook) = &cfg.before_patch {
                    hook();
                }

                // Globaler before_patch
                if let Some(hook) = &self.before_patch {
                    hook(&cfg);
                }

                // PatchElements erzeugen
                let event_res = match cfg.mode.clone() {
                    ElementPatchMode::Remove => {
                        cfg.selector.as_ref()
                            .map(|selector| {
                                PatchElements::new_remove(selector.as_str().try_into().unwrap())
                            })
                            .ok_or("selector required for Remove mode")
                    }
                    _ => {
                        cfg.content.as_ref()
                            .map(|content| {
                                let mut patch = PatchElements::new(content.as_str().try_into().unwrap());
                                
                                if let Some(selector) = &cfg.selector {
                                    patch = patch.with_selector(selector.as_str().try_into().unwrap());
                                }
                                
                                patch.with_mode(cfg.mode.clone())
                            })
                            .ok_or("content required for this mode")
                    }
                };

                match event_res {
                    Ok(event) => {
                        if cfg!(debug_assertions) {
                            sleep(Duration::from_millis(250)).await;
                        }

                        if let Some(hook) = &cfg.after_patch {
                            hook();
                        }
                        if let Some(hook) = &self.after_patch {
                            hook(&cfg);
                        }

                        yield Ok(EventData::from(event).try_into_sse_event().unwrap());
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        if let Some(hook) = &cfg.failed_patch {
                            hook(&error_msg);
                        }
                        if let Some(hook) = &self.failed_patch {
                            hook(&cfg, &error_msg);
                        }
                    }
                }
            }

            if let Some(hook) = &self.all_patched {
                hook(&self.patches);
            }
        }
    }
}

// ================= Makro =================
#[macro_export]
macro_rules! patch {
    ({ $($field:ident : $value:expr),+ $(,)? }) => {
        $crate::util::patcher::PatchConfig {
            $(
                $field: patch!(@map $field $value),
            )*
            ..Default::default()
        }
    };

    (@map before_patch $value:expr) => { Some(std::sync::Arc::new($value)) };
    (@map after_patch $value:expr) => { Some(std::sync::Arc::new($value)) };
    (@map failed_patch $value:expr) => { Some(std::sync::Arc::new($value)) };
    
    (@map selector $value:expr) => { Some($value.into()) };
    (@map content $value:expr) => { Some($value.into()) };
    (@map mode $value:expr) => { $value };
}