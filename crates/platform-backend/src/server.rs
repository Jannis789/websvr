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

    let app = Router::new_with_state(shared_state)
        // ── Public routes (no layer stack) ──
        .with_get("/login", handlers::page::login_page)
        .with_get("/register", handlers::page::register_page)
        .with_post("/login", handlers::auth::login)
        .with_post("/register", handlers::auth::register)
        .with_post("/logout", handlers::auth::logout)
        // Serve Service Worker at root (required by SW spec)
        .with_get("/sw.js", handlers::page::service_worker)
        // Serve Datastar core script
        .with_get("/assets/js/datastar-core.js", handlers::page::asset_datastar_core)
        // Serve static CSS (legacy — kept for backwards compatibility)
        .with_get("/assets/css/dark.css", handlers::page::asset_dark_css)
        .with_get("/assets/css/light.css", handlers::page::asset_light_css)
        .with_get("/assets/css/common.css", handlers::page::asset_common_css)
        // CSS assets — generic handler serves all files via URI dispatch
        // Page entry points
        .with_get("/assets/css/pages/home.css", handlers::page::asset_css)
        .with_get("/assets/css/pages/login.css", handlers::page::asset_css)
        .with_get("/assets/css/pages/register.css", handlers::page::asset_css)
        .with_get("/assets/css/pages/test.css", handlers::page::asset_css)
        // Common
        .with_get("/assets/css/common/theme.css", handlers::page::asset_css)
        .with_get("/assets/css/common/base.css", handlers::page::asset_css)
        // Features
        .with_get("/assets/css/features/window.css", handlers::page::asset_css)
        .with_get("/assets/css/features/sidebar.css", handlers::page::asset_css)
        .with_get("/assets/css/features/popup.css", handlers::page::asset_css)
        .with_get("/assets/css/features/switch.css", handlers::page::asset_css)
        .with_get("/assets/css/features/content.css", handlers::page::asset_css)
        .with_get("/assets/css/features/button.css", handlers::page::asset_css)
        .with_get("/assets/css/features/form.css", handlers::page::asset_css)
        .with_get("/assets/css/features/card.css", handlers::page::asset_css)
        .with_get("/assets/css/features/test.css", handlers::page::asset_css)
        .with_get("/assets/css/features/utility.css", handlers::page::asset_css)
        // ── Protected routes ──
        .with_sub_router_make_fn("/", |sub_router| {
            sub_router
                .with_get("/home", handlers::page::home_page)
                .with_get("/home/overview", handlers::navigate::get_home_overview)
                .with_get("/home/movies", handlers::navigate::get_home_movies)
                .with_get("/home/series", handlers::navigate::get_home_series)
                .with_get("/sse", handlers::sse_handler::sse_endpoint)
                .with_get("/test", handlers::test::test_page)
                .with_get("/test/run", handlers::test::test_run)
                .with_get("/i18n/{lang}.json", handlers::i18n_handler::i18n_json)
                .with_get("/icons/{name}.svg", handlers::icons::icon_handler)
        });

    // Build layer stack as tuple and apply to the entire app.
    // Layer order (outermost first): Compression → Auth → SessionStorage → ClientContext
    let layers = (
        CompressionLayer::new(),
        layer_fn(|inner| AuthService::new(inner)),
        layer_fn(|inner| SessionStorageService::new(inner)),
        layer_fn(|inner| ClientContextService::new(inner, sse_broadcaster.clone())),
    );
    let service = layers.layer(app);

    tracing::info!("Rama Platform server listening on http://{bind_addr}");
    tracing::info!("Layer stack: Compression → Auth → Session → ClientContext → Handler");

    HttpServer::http1()
        .listen(bind_addr, service)
        .await
        .expect("failed to start HTTP server");
}
