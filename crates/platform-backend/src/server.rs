use std::sync::Arc;

use platform_core::{Config, I18n, SseBroadcaster};
use sea_orm::DatabaseConnection;

#[derive(Debug, Clone)]
pub struct SharedState {
    pub config: &'static Config,
    pub db: DatabaseConnection,
    pub i18n: I18n,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}

impl SharedState {
    /// Initialise shared state from environment / config.
    pub async fn init() -> Self {
        let config = Config::global();

        // Load i18n from embedded JSON files
        let i18n = I18n::new(
            serde_json::from_str(include_str!("../assets/i18n/de.json"))
                .expect("Failed to parse de.json"),
            serde_json::from_str(include_str!("../assets/i18n/en.json"))
                .expect("Failed to parse en.json"),
        );

        // Load database from env; fallback to placeholder for now
        let db = crate::db::init(&config.database_url).await;

        let sse_broadcaster = Arc::new(SseBroadcaster::new(256));

        Self {
            config,
            db,
            i18n,
            sse_broadcaster,
        }
    }
}

/// Start the HTTP server with routing and layer stack.
pub async fn run() {
    use rama::http::server::HttpServer;
    use rama::http::service::web::Router;
    use rama::http::layer::compression::CompressionLayer;
    use rama::layer::layer_fn;
    use rama::Layer;

    use crate::layers::auth::AuthService;
    use crate::layers::session_storage::SessionStorageService;
    use crate::layers::client_context::ClientContextService;
    use crate::handlers;

    let shared_state = SharedState::init().await;
    let bind_addr = format!("{}:{}", shared_state.config.host, shared_state.config.port);
    let sse_broadcaster = shared_state.sse_broadcaster.clone();

    // ── Protected sub-router with its own layer stack ──
    let protected = Router::new_with_state(shared_state.clone())
        .with_get("/home", handlers::page::home_page)
        .with_get("/home/overview", handlers::navigate::get_home_overview)
        .with_get("/home/movies", handlers::navigate::get_home_movies)
        .with_get("/home/series", handlers::navigate::get_home_series)
        .with_get("/sse", handlers::sse_handler::sse_endpoint)
        .with_get("/test", handlers::test::test_page)
        .with_get("/test/run", handlers::test::test_run)
        .with_get("/i18n/{lang}.json", handlers::i18n_handler::i18n_json);

    // Apply Auth/Session/ClientContext layers only to protected routes
    let protected_layers = (
        layer_fn(|inner| AuthService::new(inner)),
        layer_fn(|inner| SessionStorageService::new(inner)),
        layer_fn(|inner| ClientContextService::new(inner, sse_broadcaster.clone())),
    );
    let protected_service = protected_layers.layer(protected);

    // ── Public routes (no layers) ──
    let app = Router::new_with_state(shared_state)
        // Static assets
        .with_dir("/assets", std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"))
        // Public pages
        .with_get("/login", handlers::page::login_page)
        .with_get("/register", handlers::page::register_page)
        .with_post("/login", handlers::auth::login)
        .with_post("/register", handlers::auth::register)
        .with_post("/logout", handlers::auth::logout)
        // Service Worker must be at root scope per SW spec
        .with_file("/sw.js", std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/js/sw.js"), "application/javascript".parse().unwrap())
        // Protected routes with layer stack
        .with_sub_service("/", protected_service);

    tracing::info!("Rama Platform server listening on http://{bind_addr}");

    let service = CompressionLayer::new().layer(app);

    HttpServer::http1()
        .listen(bind_addr, service)
        .await
        .expect("failed to start HTTP server");
}
