use crate::elog;
use std::sync::Arc;

use platform_core::{Config, I18n};
use crate::sse::SseBroadcaster;
use sea_orm::DatabaseConnection;
use rama::http::layer::compression::CompressionLayer;
use rama::http::service::web::Router;
use rama::Layer;

use crate::layers::{auth, session_stack};

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

// ── Server ────────────────────────────────────────────────────

pub async fn run() {
    use rama::http::server::HttpServer;
    use crate::handlers;

    let state = SharedState::init().await;
    let bind = format!("{}:{}", state.config.host, state.config.port);
    let broadcaster = state.sse_broadcaster.clone();

    let app = Router::new_with_state(state.clone())
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
        // Everything below needs a session
        .with_sub_service("/", session_stack::session_layer(broadcaster, Router::new_with_state(state.clone())
            // Auth-required: redirect to /login if not authenticated
            .with_sub_service("/", auth::require_auth(Router::new_with_state(state.clone())
                .with_get("/home", handlers::pages::home_page)
                .with_get("/home/overview", handlers::navigate::get_home_overview)
                .with_get("/home/movies", handlers::navigate::get_home_movies)
                .with_get("/home/series", handlers::navigate::get_home_series)
                .with_get("/sse", handlers::sse_handler::sse_endpoint)
            ))
            // Session-only: no auth check
            .with_get("/test", handlers::test::test_page)
            .with_get("/test/run", handlers::test::test_run)
            .with_get("/i18n/{lang}.json", handlers::i18n_handler::i18n_json)
            .with_post("/login", handlers::auth::login)
            .with_post("/register", handlers::auth::register)
            .with_post("/logout", handlers::auth::logout)
        ));

    elog!(Info, "Rama Platform server listening on http://{bind}");

    let service = CompressionLayer::new().layer(app);
    HttpServer::http1()
        .listen(bind, service)
        .await
        .expect("failed to start HTTP server");
}
