use async_stream::stream;
use rama::http::service::web::WebService;
use rama::http::service::web::response::{Html, Sse, IntoResponse};
use rama::http::sse::Event;
use rama::http::sse::server::{KeepAlive, KeepAliveStream};
use std::convert::Infallible;

pub struct HomePage;

impl HomePage {
    pub fn mount(svc: WebService<()>) -> WebService<()> {
        svc
            .with_get("/", Self::index)
            .with_get("/HomePage/sse", Self::patch_stream)
    }

    async fn index() -> Html<&'static str> {
        Html(HomePage::get_layout_template())
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
                <div id="footer"></div>
            </body>
            </html>
        "#
    }

    async fn patch_stream() -> impl IntoResponse {
        let stream = stream! {
            // WICHTIG: HTML MUSS DIE GLEICHE ID HABEN WIE IM DOM!
            let header_html = remove_line_breaks(r#"<div id="header" datastar-patch="replace">
            test
            </div>"#);

            // KORREKTES DATSTAR-EVENT-FORMAT:
            // 1. Event-Typ: "datastar-patch-elements"
            // 2. Daten beginnen mit "elements "
            let event = Event::default()
                .try_with_event("datastar-patch-elements") // ⭐ OFFIZIELLER EVENT-TYP
                .expect("Fehler beim Setzen des Event-Typs")
                .with_data(format!("elements {}", remove_empty_lines(&header_html))); // ⭐ "elements " PREFIX
            
            yield Ok::<_, Infallible>(event);
        };

        let keep_alive = KeepAlive::new();
        let keep_alive_stream = KeepAliveStream::new(keep_alive, stream);
        
        Sse::new(keep_alive_stream)
    }
}

fn remove_empty_lines(input: &str) -> String {
    input
        .lines()                    // String in Zeilen aufteilen
        .filter(|line| !line.trim().is_empty()) // nur nicht-leere Zeilen behalten
        .collect::<Vec<&str>>()     // zurück in einen Vec
        .join("\n")                 // wieder zu einem String zusammenfügen
}

fn remove_line_breaks(input: &str) -> String {
    input.replace("\r\n", "").replace('\n', "")
}