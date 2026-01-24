pub struct Footer;

impl Footer {
    pub fn get_template() -> &'static str {
        "<footer>Footer</footer>"
    }

    pub fn render() -> &'static str {
        Self::get_template()
    }
}