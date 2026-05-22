use rama::service::Service;
use rama::http::Request;
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::response::Response;
use platform_core::{ClientId, SessionStorage};
use std::convert::Infallible;
use std::future::Future;

/// Service that rehydrates or creates a `SessionStorage` and injects it
/// into `req.extensions()`.
///
/// Reads `ClientId` from extensions (injected by `AuthService`),
/// loads persisted session data from DB, creates fresh if none exists.
///
/// Wrapped as a layer via `rama::layer::layer_fn`.
#[derive(Debug, Clone)]
pub struct SessionStorageService<S> {
    inner: S,
}

impl<S> SessionStorageService<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ResBody> Service<Request> for SessionStorageService<S>
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

            // For now, create a fresh SessionStorage.
            // In a later phase, load persisted data from DB.
            let session = SessionStorage::new(client_id);

            let mut req = req;
            req.extensions_mut().insert(session);

            Ok(self.inner.serve(req).await.unwrap())
        }
    }
}
