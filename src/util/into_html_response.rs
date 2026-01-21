use rama::http::service::web::response::Html;

pub fn into_html_response(s: &'static str) -> Html<&'static str> {
    Html(s)
}