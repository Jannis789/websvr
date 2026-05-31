use crate::context::ClientContext;
use rama::extensions::ExtensionsRef;
use rama::http::header;
use rama::http::Request;

/// Extract `ClientContext` from request extensions.
#[inline]
pub fn extract_context(req: &Request) -> ClientContext {
    req.extensions()
        .get::<ClientContext>()
        .cloned()
        .expect("ClientContext must be injected by ClientContextService layer")
}

/// Extract a cookie value by name from the request's Cookie header.
pub fn get_cookie_value(req: &Request, cookie_name: &str) -> Option<String> {
    let cookie_header = req.headers().get(header::COOKIE)?.to_str().ok()?;
    cookie_header.split(';').map(|s| s.trim()).find_map(|pair| {
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
