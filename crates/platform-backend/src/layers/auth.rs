use rama::service::Service;
use rama::http::Request;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::response::Response;
use platform_core::{ClientId, Config};
use std::convert::Infallible;
use std::future::Future;

use rama::http::header;

use crate::common;

/// Service that validates or generates a `ClientId` from the request cookie.
///
/// If the `platform_cid` cookie is present and valid, its value is injected
/// into `req.extensions()`.  Otherwise a fresh `ClientId` is generated and
/// a `Set-Cookie` header is prepared.
///
/// Wrapped as a layer via `rama::layer::layer_fn`.
#[derive(Debug, Clone)]
pub struct AuthService<S> {
    inner: S,
}

impl<S> AuthService<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ResBody> Service<Request> for AuthService<S>
where
    S: Service<Request, Output = Response<ResBody>, Error = Infallible>,
    ResBody: Default + From<String> + Send + 'static,
{
    type Output = Response<ResBody>;
    type Error = Infallible;

    fn serve(
        &self,
        req: Request,
    ) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send + '_ {
        async move {
            let had_cookie = common::get_cookie_value(&req, platform_core::client_id::CLIENT_ID_COOKIE).is_some();
            let client_id = extract_or_generate_client_id(&req);
            tracing::debug!("AuthService → client_id={} (from_cookie={})", client_id, had_cookie);
            let mut req = req;
            req.extensions_mut().insert(client_id);

            let mut response = self.inner.serve(req).await.unwrap();

            // Set cookie only if it wasn't already present
            if !had_cookie {
                let ttl_days = Config::global().client_id_ttl_days;
                let max_age = ttl_days as u64 * 24 * 60 * 60;
                tracing::debug!("AuthService → new cookie set for {} (TTL={}d)", client_id, ttl_days);
                let cookie = format!(
                    "{}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                    platform_core::client_id::CLIENT_ID_COOKIE,
                    client_id,
                    max_age,
                );
                response.headers_mut().insert(
                    header::SET_COOKIE,
                    cookie.parse().unwrap(),
                );
            }

            Ok(response)
        }
    }
}

fn extract_or_generate_client_id(req: &Request) -> ClientId {
    // Try to parse from extensions first (already set by a higher layer)
    if let Some(cid) = req.extensions().get::<ClientId>() {
        return *cid;
    }

    // Try to parse from cookie
    if let Some(cookie_str) = common::get_cookie_value(req, platform_core::client_id::CLIENT_ID_COOKIE) {
        if let Some(cid) = ClientId::parse(&cookie_str) {
            return cid;
        }
    }

    // Generate new
    ClientId::generate()
}
