use crate::elog;
use std::sync::Arc;

use platform_core::{ClientId, Config, I18n};
use crate::sse::SseBroadcaster;
use sea_orm::DatabaseConnection;
use rama::http::layer::compression::CompressionLayer;
use rama::http::layer::validate_request::ValidateRequestHeaderLayer;
use rama::layer::layer_fn;
use rama::Layer;
use rama::extensions::{ExtensionsMut, ExtensionsRef};

use crate::common;
use crate::layers::session_storage::SessionStorageService;
use crate::layers::client_context::{ClientContextService, CookieWasPresent};

// ── Shared State ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SharedState {
    pub config: &'static Config,
    pub db: DatabaseConnection,
    pub i18n: I18n,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}

impl SharedState {
    pub async fn init() -> Self {
        let config = Config::global();
        let i18n = I18n::new(
            serde_json::from_str(include_str!("../assets/i18n/de.json"))
                .expect("Failed to parse de.json"),
            serde_json::from_str(include_str!("../assets/i18n/en.json"))
                .expect("Failed to parse en.json"),
        );
        let db = crate::db::init(&config.database_url).await;
        let sse_broadcaster = Arc::new(SseBroadcaster::new(256));
        Self { config, db, i18n, sse_broadcaster }
    }
}

// ── Server Entry ──────────────────────────────────────────────

pub async fn run() {
    use rama::http::server::HttpServer;
    use rama::http::service::web::Router;
    use crate::handlers;

    let shared_state = SharedState::init().await;
    let bind_addr = format!("{}:{}", shared_state.config.host, shared_state.config.port);
    let broadcaster = shared_state.sse_broadcaster.clone();

    // ── Auth-required routes (/home, /home/*, /sse) ────────
    //
    // Layer stack: ClientId → Session → ClientContext → Auth
    // Redirects to /login if session lacks "authenticated" flag.
    let auth_router = Router::new_with_state(shared_state.clone())
        .with_get("/home", handlers::pages::home_page)
        .with_get("/home/overview", handlers::navigate::get_home_overview)
        .with_get("/home/movies", handlers::navigate::get_home_movies)
        .with_get("/home/series", handlers::navigate::get_home_series)
        .with_get("/sse", handlers::sse_handler::sse_endpoint);

    let auth_stack = (
        // Layer 1: Extract or generate ClientId from cookie
        ValidateRequestHeaderLayer::custom_fn(|mut req: rama::http::Request| async move {
            let had_cookie = common::get_cookie_value(&req, platform_core::client_id::CLIENT_ID_COOKIE).is_some();
            let client_id = extract_or_generate_client_id(&req);
            elog!(Debug, "ClientId → {} (cookie={})", client_id, had_cookie);
            req.extensions_mut().insert(client_id);
            req.extensions_mut().insert(CookieWasPresent(had_cookie));
            Ok(req)
        }),
        // Layer 2: Load or create session
        layer_fn(|inner| SessionStorageService::new(inner)),
        // Layer 3: Assemble ClientContext + set cookie for new clients
        layer_fn(|inner| ClientContextService::new(inner, broadcaster.clone())),
        // Layer 4: Auth — check session for "authenticated", redirect to /login if missing
        ValidateRequestHeaderLayer::custom_fn(|req: rama::http::Request| async move {
            let authenticated = req.extensions()
                .get::<crate::client_context::ClientContext>()
                .and_then(|ctx| ctx.session_storage.get("authenticated"))
                .and_then(|v: &serde_json::Value| v.as_bool())
                .unwrap_or(false);

            if authenticated {
                Ok(req)
            } else {
                elog!(Debug, "Auth → redirecting to /login");
                let resp = rama::http::Response::builder()
                    .status(rama::http::StatusCode::SEE_OTHER)
                    .header("location", "/login")
                    .body(rama::http::Body::empty())
                    .unwrap();
                Err(resp)
            }
        }),
    );
    let auth_service = auth_stack.layer(auth_router);

    // ── Session routes (/login forms, /test — Session but no auth) ──
    //
    // Layer stack: ClientId → Session → ClientContext
    let session_router = Router::new_with_state(shared_state.clone())
        .with_get("/test", handlers::test::test_page)
        .with_get("/test/run", handlers::test::test_run)
        .with_get("/i18n/{lang}.json", handlers::i18n_handler::i18n_json)
        .with_post("/login", handlers::auth::login)
        .with_post("/register", handlers::auth::register)
        .with_post("/logout", handlers::auth::logout);

    let session_stack = (
        ValidateRequestHeaderLayer::custom_fn(|mut req: rama::http::Request| async move {
            let had_cookie = common::get_cookie_value(&req, platform_core::client_id::CLIENT_ID_COOKIE).is_some();
            let client_id = extract_or_generate_client_id(&req);
            req.extensions_mut().insert(client_id);
            req.extensions_mut().insert(CookieWasPresent(had_cookie));
            Ok(req)
        }),
        layer_fn(|inner| SessionStorageService::new(inner)),
        layer_fn(|inner| ClientContextService::new(inner, broadcaster.clone())),
    );
    let session_service = session_stack.layer(session_router);

    // ── Public routes (no layers) ───────────────────────────
    let app = Router::new_with_state(shared_state)
        // Public pages
        .with_get("/login", handlers::pages::login_page)
        .with_get("/register", handlers::pages::register_page)
        // Static assets
        .with_dir("/assets", std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"))
        .with_file(
            "/sw.js",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/js/sw.js"),
            "application/javascript".parse().unwrap(),
        )
        // Sub-routers
        .with_sub_service("/", auth_service)
        .with_sub_service("/", session_service);

    elog!(Info, "Rama Platform server listening on http://{bind_addr}");

    let service = CompressionLayer::new().layer(app);
    HttpServer::http1()
        .listen(bind_addr, service)
        .await
        .expect("failed to start HTTP server");
}

// ── Helpers ───────────────────────────────────────────────────

fn extract_or_generate_client_id(req: &rama::http::Request) -> ClientId {
    if let Some(cid) = req.extensions().get::<ClientId>() {
        return *cid;
    }
    if let Some(cookie_str) = common::get_cookie_value(req, platform_core::client_id::CLIENT_ID_COOKIE) {
        if let Some(cid) = ClientId::parse(&cookie_str) {
            return cid;
        }
    }
    ClientId::generate()
}
