use std::sync::Arc;

use rama::service::Service;
use rama::http::Request;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::response::Response;
use platform_core::{ClientId, SessionStorage, ClientContext, SseBroadcaster};
use std::convert::Infallible;
use std::future::Future;

/// Service that aggregates `ClientId` and `SessionStorage` into a
/// `ClientContext` and injects it into `req.extensions()`.
///
/// Also attaches the `Arc<SseBroadcaster>` (from `SharedState`) so
/// handlers can emit SSE events directly from the context.
///
/// Wrapped as a layer via `rama::layer::layer_fn`.
#[derive(Debug, Clone)]
pub struct ClientContextService<S> {
    inner: S,
    sse_broadcaster: Arc<SseBroadcaster>,
}

impl<S> ClientContextService<S> {
    pub fn new(inner: S, sse_broadcaster: Arc<SseBroadcaster>) -> Self {
        Self { inner, sse_broadcaster }
    }
}

impl<S, ResBody> Service<Request> for ClientContextService<S>
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
            let client_id = req.extensions()
                .get::<ClientId>()
                .copied()
                .expect("ClientId must be injected by AuthService");

            let session = req.extensions()
                .get::<SessionStorage>()
                .cloned()
                .expect("SessionStorage must be injected by SessionStorageService");

            let ctx = ClientContext::with_session(
                client_id,
                session,
                self.sse_broadcaster.clone(),
            );

            let mut req = req;
            req.extensions_mut().insert(ctx);
            crate::elog!(Ok, "ClientContext → assembled for client_id={}", client_id);

            Ok(self.inner.serve(req).await.unwrap())
        }
    }
}
