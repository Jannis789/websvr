use rama::http::sse::datastar::PatchElements;
use rama::utils::str::NonEmptyStr;

pub fn patch_element(id: &str, html: &str) -> PatchElements {
    let html = format!(r#"<div id="{id}">{html}</div>"#);

    PatchElements::new(
        NonEmptyStr::try_from(html.as_str()).expect("html must not be empty")
    )
}