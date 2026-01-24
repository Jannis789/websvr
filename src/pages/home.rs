use rama::http::{
    service::web::{ extract::Query, WebService, response::{ Html, IntoResponse, Json, Sse } },
    sse::datastar::ElementPatchMode,
};

use serde::Deserialize;
use serde_json::json;

use crate::{ patch, util::patcher::{ PatchConfig, Patcher }, components::{ Header, Body, Footer } };

#[derive(Deserialize)]
struct HeadParams {
    dark: bool,
}

pub struct HomePage;

impl HomePage {
    pub fn mount(svc: WebService<()>) -> WebService<()> {
        svc.with_get("/", Html(HomePage::get_template()))
            .with_get("/HomePage/theme", Self::deploy_css)
            .with_get("/HomePage/sse", Self::patch_stream)
    }

    fn get_template() -> &'static str {
        r#"
            <!doctype html>
            <html>
            <head data-init="@get('/HomePage/theme?dark=' + window.matchMedia('(prefers-color-scheme: dark)').matches)">
                <meta charset="utf-8">
                <title>Home</title>
                <style data-signals:css="''" data-text="$css"></style>
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

    async fn patch_stream() -> impl IntoResponse {
        let stream = Patcher::new().set(
            vec![
                patch!({
                mode: ElementPatchMode::Replace,
                content: format!(r#"<div id="header">{}</div>"#,Header::get_template()),
            }),
                patch!({
                mode: ElementPatchMode::Replace,
                content: format!(r#"<div id="body">{}</div>"#,Body::get_template()),
            }),
                patch!({
                mode: ElementPatchMode::Replace,
                content: format!(r#"<div id="footer">{}</div>"#,Footer::get_template()),
            })
            ]
        );

        Sse::new(stream.build())
    }

    async fn deploy_css(Query(params): Query<HeadParams>) -> impl IntoResponse {
        let mut css = "".to_string(); 
        if params.dark {
            css += include_str!("../../public/typography/colors-dark.css");
        } else {
            css += include_str!("../../public/typography/colors-light.css");
        }
        css += include_str!("../../public/typography/component.css");

        Json(json!({ "css": css }))
    }
}
