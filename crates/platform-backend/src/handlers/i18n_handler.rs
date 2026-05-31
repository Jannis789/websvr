use crate::context::SharedState;
use rama::http::header;
use rama::http::service::web::extract::State;
use rama::http::{Request, Response, StatusCode};

/// GET /i18n/{lang}.json — serve translation JSON
pub async fn i18n_json(State(state): State<SharedState>, req: Request) -> Response {
    let path = req.uri().path();
    let lang = path
        .strip_prefix("/i18n/")
        .and_then(|s| s.strip_suffix(".json"))
        .unwrap_or("en");

    let json = match lang {
        "de" => &state.i18n.get(platform_core::Lang::De),
        _ => &state.i18n.get(platform_core::Lang::En),
    };

    let mut resp = Response::new(rama::http::Body::from(json.to_string()));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "application/json".parse().expect("invalid header value"),
    );
    resp
}
