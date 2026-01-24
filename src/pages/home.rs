use rama::http::{
    service::web::{ WebService, response::{ Html, IntoResponse, Sse } },
    sse::datastar::ElementPatchMode,
};

use crate::{ patch, util::patcher::{ Patcher }, components::{ Header, Body, Footer } };

pub struct HomePage;

impl HomePage {
    pub fn serve(svc: WebService<()>) -> WebService<()> {
        svc.with_get("/", Html(HomePage::get_template()))
            .with_get("/HomePage/sse", Self::handle_patch)
    }

    fn get_template() -> &'static str {
        r#"
            <!doctype html>
            <html>
                <head>
                  <meta charset="utf-8">
                  <title>Home</title>
                  <link rel="stylesheet" href="/public/typography/base.css">
                  <script>
                    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
                    document.write(
                      `<link rel="stylesheet" href="/public/typography/colors-${dark ? "dark" : "light"}.css">`
                    );
                  </script>
                  <link rel="stylesheet" href="/public/pages/home.css">
                  <script type="module" src="https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.0-RC.7/bundles/datastar.js"></script>
                </head>
                <body data-init="@get('/HomePage/sse')">
                    <div id="header"></div>
                    <div id="body"></div>
                    <div id="footer"></div>
                </body>
            </html>
        "#
    }

    async fn handle_patch() -> impl IntoResponse {
        let stream = Patcher::new().set(
            vec![
                patch!({
                mode: ElementPatchMode::Replace,
                content: format!(r#"<div id="header">{}</div>"#,Header::render()),
            }),
                patch!({
                mode: ElementPatchMode::Replace,
                content: format!(r#"<div id="body">{}</div>"#,Body::render()),
            }),
                patch!({
                mode: ElementPatchMode::Replace,
                content: format!(r#"<div id="footer">{}</div>"#,Footer::render()),
            })
            ]
        );

        Sse::new(stream.build())
    }
}
