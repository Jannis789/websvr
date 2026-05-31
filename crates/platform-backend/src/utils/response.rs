use rama::http::header;
use rama::http::StatusCode;

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
    resp.headers_mut()
        .insert(header::LOCATION, url.parse().expect("invalid header value"));
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
