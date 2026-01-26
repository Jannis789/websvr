use crate::util::component::Component;
use std::collections::HashMap;
use serde_json::Value;

pub struct Sidebar;

impl Component for Sidebar {
    fn get_template() -> &'static str {
        r#"
        <nav id="sidebar">
            <div class="sidebar-header">
                <button class="btn btn-ghost">
                    <i class="ph ph-list text-xl"></i>
                </button>
            </div>
            
            <div class="sidebar-footer">
            </div>
        </nav>
        "#
    }

    fn render(&self, _params: Option<&HashMap<String, Value>>) -> String {
        Self::get_template().to_string()
    }
}
