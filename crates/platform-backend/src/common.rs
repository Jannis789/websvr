use rama::http::{Request, StatusCode};
use rama::http::header;
use rama::extensions::ExtensionsRef;
use platform_core::ClientContext;

/// Utility: extract `ClientContext` from request extensions.
#[inline]
pub fn extract_context(req: &Request) -> ClientContext {
    req.extensions()
        .get::<ClientContext>()
        .cloned()
        .expect("ClientContext must be injected by ClientContextService layer")
}

/// Common response type for all handlers.
pub type Response = rama::http::Response<rama::http::Body>;

pub fn empty_response(status: StatusCode) -> Response {
    let mut resp = Response::new(rama::http::Body::empty());
    *resp.status_mut() = status;
    resp
}

pub fn redirect(url: &str) -> Response {
    let mut resp = Response::new(rama::http::Body::empty());
    *resp.status_mut() = StatusCode::SEE_OTHER;
    resp.headers_mut().insert(
        header::LOCATION,
        url.parse().expect("invalid header value"),
    );
    resp
}

pub fn html_response(html: &str) -> Response {
    let mut resp = Response::new(rama::http::Body::from(html.to_string()));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8".parse().expect("invalid header value"),
    );
    resp
}

/// Build an SSE response containing a single `datastar-patch-elements` event.
/// Uses Rama's PatchElements + write_data — no manual SSE string building.
pub fn sse_patch_response(selector: &str, html: &str) -> Response {
    use rama::http::body::sse::datastar::PatchElements;
    use rama::http::body::sse::EventDataWrite;

    let patch = PatchElements::new(html.try_into().unwrap())
        .with_selector(selector.try_into().unwrap());

    let mut data_buf = Vec::new();
    patch.write_data(&mut data_buf).unwrap();
    let data_lines = String::from_utf8(data_buf).unwrap();

    let body = format!("event: datastar-patch-elements\nid: nav\n{data_lines}\n");

    let mut resp = Response::new(rama::http::Body::from(body));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/event-stream; charset=utf-8".parse().unwrap(),
    );
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-cache".parse().unwrap(),
    );
    resp
}

/// Extract a cookie value by name from the request's Cookie header.
pub fn get_cookie_value(req: &Request, cookie_name: &str) -> Option<String> {
    let cookie_header = req.headers().get(header::COOKIE)?.to_str().ok()?;
    cookie_header
        .split(';')
        .map(|s| s.trim())
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let name = parts.next()?.trim();
            let value = parts.next()?.trim();
            if name == cookie_name {
                Some(value.to_string())
            } else {
                None
            }
        })
}
