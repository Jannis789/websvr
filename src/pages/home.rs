use rama::http::{
    service::web::{ WebService, response::{ Html, IntoResponse, Sse } },
    sse::datastar::ElementPatchMode,
};
use crate::{ component_patch, util::component_patcher::{ ComponentPatcher }, components::layout::{ Header, Body, Sidebar } };
pub struct HomePage;

impl HomePage {
    pub fn serve(svc: WebService<()>) -> WebService<()> {
        svc.with_get("/", Html(HomePage::get_template()))
            .with_get("/HomePage/sse", Self::handle_patch)
    }

    fn get_template() -> &'static str {
        r#"
            <!doctype html>
            <html lang="de">
                <head>
                  <meta charset="utf-8">
                  <meta name="viewport" content="width=device-width, initial-scale=1.0">
                  <title>Media Server</title>
                  <!-- Icons (Phosphor) -->
                  <script src="https://unpkg.com/@phosphor-icons/web"></script>
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
                    <div id="main-container">
                        <div id="sidebar"></div>
                        <div id="header"></div>
                        <div id="body"></div>
                    </div>
                </body>
            </html>
        "#
    }

    async fn handle_patch() -> impl IntoResponse {
        let stream = ComponentPatcher::new().set(
            vec![
                component_patch!({
                    mode: ElementPatchMode::Inner,
                    selector: "#sidebar",
                    content: Sidebar,
                }),
                component_patch!({
                    mode: ElementPatchMode::Inner,
                    selector: "#header",
                    content: Header,
                }),
                component_patch!({
                    mode: ElementPatchMode::Inner,
                    selector: "#body",
                    content: Body,
                }),
            ]
        );

        Sse::new(stream.build())
    }
}
