pub struct Body;

impl Body {
    fn get_template() -> &'static str {
        r#"<div id="body-content">Body</div>"#
    }

    pub fn render() -> &'static str {
        Self::get_template()
    }
}