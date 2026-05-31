use crate::elog;
use platform_core::{ClientId, SessionStorage};
use rama::extensions::{ExtensionsMut, ExtensionsRef};
use rama::http::response::Response;
use rama::http::Request;
use rama::service::Service;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Shared in-memory session store. Keyed by ClientId.
pub type SessionMap = Arc<Mutex<HashMap<ClientId, Arc<Mutex<SessionStorage>>>>>;

/// Create the shared session map (call once in routes::run).
pub fn new_session_map() -> SessionMap {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Service that rehydrates or creates a shared `SessionStorage`
/// and injects it into request extensions.
#[derive(Debug, Clone)]
pub struct SessionStorageService<S> {
    inner: S,
    sessions: SessionMap,
}

impl<S> SessionStorageService<S> {
    pub fn new(inner: S, sessions: SessionMap) -> Self {
        Self { inner, sessions }
    }
}

impl<S, ResBody> Service<Request> for SessionStorageService<S>
where
    S: Service<Request, Output = Response<ResBody>, Error = Infallible>,
    ResBody: Default + From<String> + Send + 'static,
{
    type Output = Response<ResBody>;
    type Error = Infallible;

    fn serve(&self, req: Request) -> impl Future<Output = Result<Self::Output, Self::Error>> + Send + '_ {
        let sessions = self.sessions.clone();
        async move {
            let client_id = req
                .extensions()
                .get::<ClientId>()
                .copied()
                .expect("ClientId must be injected by preceding layer");

            // Load existing or create fresh shared session
            let session: Arc<Mutex<SessionStorage>> = {
                let mut map = sessions.lock().await;
                map.entry(client_id)
                    .or_insert_with(|| {
                        elog!(Debug, "SessionStorage → new for {}", client_id);
                        Arc::new(Mutex::new(SessionStorage::new(client_id)))
                    })
                    .clone()
            };

            // Inject the Arc<Mutex<SessionStorage>> into request
            let mut req = req;
            req.extensions_mut().insert(session);

            self.inner.serve(req).await
        }
    }
}
