use rama::http::service::web::WebService;
use rama::http::service::web::response::{ Html, Sse, IntoResponse };
use crate::util::apply_patch::{Patcher, PatchConfig};
use rama::http::sse::datastar::ElementPatchMode;
use crate::patch;
pub struct HomePage;

impl HomePage {
    pub fn mount(svc: WebService<()>) -> WebService<()> {
        svc.with_get("/", Html(HomePage::get_layout_template())).with_get(
            "/HomePage/sse",
            Self::patch_stream
        )
    }

    fn get_layout_template() -> &'static str {
        r#"
            <!doctype html>
            <html>
            <head>
                <meta charset="utf-8">
                <title>Home</title>
                <!-- URL OHNE LEERZEICHEN am Ende! -->
                <script type="module" src="https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.7/bundles/datastar.js"></script>
            </head>
            <body data-init="@get('/HomePage/sse')">
                <div id="header"></div>
                <div id="body"></div>
                <div id="footer">x</div>
            </body>
            </html>
        "#
    }

    async fn patch_stream() -> impl IntoResponse {
        let stream = Patcher::new().set(vec![
            patch!({
                selector: "#footer",
                mode: ElementPatchMode::Remove,
            }),
            patch!({
                mode: ElementPatchMode::Replace,
                content: r#"<div id="header"><h1>Willkommen auf der Startseite!</h1></div>"#,
            }),
            patch!({
                selector: "#body",
                mode: ElementPatchMode::Inner,
                content: r#"<p id="body">Neuer Absatz</p>"#,
            }),
        ]);

        Sse::new(stream.build())
    }
}

