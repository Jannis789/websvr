use crate::util::component::Component;
use std::collections::HashMap;
use serde_json::Value;

pub struct Sidebar;

impl Component for Sidebar {
    fn get_template() -> &'static str {
        r#"
        <nav id="sidebar-content" data-signals:collapsed="false" data-class:collapsed="$collapsed"> 
            <div class="sidebar-header">
                <button class="btn btn-ghost" data-on:click="$collapsed = !$collapsed">
                    <i class="ic-open-menu ic-medium"></i>
                </button>
            </div>
        </nav>
        "#
    }

    fn render(&self, _params: Option<&HashMap<String, Value>>) -> String {
        Self::get_template().to_string()
    }
}
