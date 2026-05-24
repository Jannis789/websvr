use rama::http::{Request, StatusCode};
use rama::http::header;
use rama::extensions::ExtensionsRef;
use crate::client_context::ClientContext;

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
