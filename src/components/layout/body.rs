use crate::util::component::Component;
use std::collections::HashMap;
use serde_json::Value;

pub struct Body;

impl Component for Body {
    fn get_template() -> &'static str {
        r#"
        <div></div>
        "#
    }

    fn render(&self, _params: Option<&HashMap<String, Value>>) -> String {
        Self::get_template().to_string()
    }
}
