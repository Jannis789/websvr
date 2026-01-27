use crate::util::component::Component;
use std::collections::HashMap;
use serde_json::Value;

pub struct Header;

impl Component for Header {
    fn get_template() -> &'static str {
        r#"
<header>
    <div class="header-spacer"></div>
    <div class="header-controls" data-class:collapsed="$collapsed">
        <div class="header-controls-left">
        </div>
        <div class="header-controls-middle">
            <h2>Media Server UI</h2>
        </div>
        <div class="header-controls-right">
            <button class="btn btn-ghost">
            <i class="ic-search ic-medium"></i>
            </button>
            <div class="header-separator"></div>
            <button class="btn btn-ghost">
                <i class="ic-notification-bell ic-medium"></i>
            </button>
            <button class="btn btn-ghost">
                <i class="ic-avatar-default ic-medium"></i>
            </button>
        </div>
    </div>
</header>
"#
    }

    fn render(&self, _params: Option<&HashMap<String, Value>>) -> String {
        Self::get_template().to_string()
    }
}
