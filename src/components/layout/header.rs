use crate::util::component::Component;
use std::collections::HashMap;
use serde_json::Value;

pub struct Header;

impl Component for Header {
    fn get_template() -> &'static str {
        r#"
<header id="header">
    <div class="header-spacer"></div>
    <div class="header-controls">
        <button class="btn btn-ghost">
            <i class="ph ph-magnifying-glass text-xl"></i>
        </button>
        <div class="header-separator"></div>
        <button class="btn btn-ghost">
            <i class="ph ph-bell text-xl"></i>
        </button>
        <button class="btn btn-ghost">
            <i class="ph ph-user text-xl"></i>
        </button>
    </div>
</header>
"#
    }

    fn render(&self, _params: Option<&HashMap<String, Value>>) -> String {
        Self::get_template().to_string()
    }
}
