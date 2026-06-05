use crate::elog;
use rama::http::layer::compression::CompressionLayer;
use rama::http::service::web::extract::State;
use rama::http::service::web::Router;
use rama::http::Request;
use rama::Layer;

use crate::context::SharedState;
use crate::layers::{auth, session_stack};

// ── Server ────────────────────────────────────────────────────

pub async fn run() {
    use crate::handlers;
    use rama::http::server::HttpServer;

    let state = SharedState::init().await;
    let bind = format!("{}:{}", state.config.host, state.config.port);
    let server_epoch = state.server_epoch;

    let app = Router::new_with_state(state.clone())
        // Static assets
        .with_dir(
            "/assets",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"),
        )
        .with_get(
            "/sw.js",
            |State(_state): State<SharedState>, _req: Request| async move {
               let sw_js = include_str!("../assets/js/sw.js");
               let mut resp = crate::utils::response::Response::new(rama::http::Body::from(sw_js));
               *resp.status_mut() = rama::http::StatusCode::OK;
                resp.headers_mut().insert(
                    rama::http::header::CONTENT_TYPE,
                    "application/javascript".parse().unwrap(),
                );
                resp.headers_mut().insert(
                    rama::http::header::CACHE_CONTROL,
                    "no-cache".parse().unwrap(),
                );
                resp
            },
        )
        .with_get(
            "/sw2.js",
            |State(_state): State<SharedState>, _req: Request| async move {
               let sw_js = include_str!("../assets/js/sw.js");
               let mut resp = crate::utils::response::Response::new(rama::http::Body::from(sw_js));
               *resp.status_mut() = rama::http::StatusCode::OK;
                resp.headers_mut().insert(
                    rama::http::header::CONTENT_TYPE,
                    "application/javascript".parse().unwrap(),
                );
                resp.headers_mut().insert(
                    rama::http::header::CACHE_CONTROL,
                    "no-cache".parse().unwrap(),
                );
                resp
            },
        )
        // Everything below needs a session
        .with_sub_service(
            "/",
            session_stack::session_layer(
                server_epoch,
                Router::new_with_state(state.clone())
                    // Auth-required
                    .with_sub_service(
                        "/",
                        auth::require_auth(
                            Router::new_with_state(state.clone())
                                .with_get("/home", handlers::pages::home_page)
                                .with_get("/home/overview", handlers::navigate::get_home_overview)
                                .with_get("/home/movies", handlers::navigate::get_home_movies)
                                .with_get("/home/series", handlers::navigate::get_home_series)
                                .with_get("/settings", handlers::pages::settings_page)
                                .with_get("/settings/account", handlers::pages::get_settings_account),
                        ),
                    )
                    // Session-only: no auth check
                    .with_get("/sse", handlers::sse_handler::sse_endpoint)
                    .with_get("/login", handlers::pages::login_page)
                    .with_get("/register", handlers::pages::register_page)
                    .with_get("/test", handlers::test::test_page)
                    .with_get("/test/auth", handlers::test::test_auth)
                    .with_get("/test/run", handlers::test::test_run)
                    .with_get("/test/1", handlers::test::test_action)
                    .with_get("/test/clear", handlers::test::test_clear)
                    .with_get("/test/stats", handlers::test::test_stats)
                    .with_get("/i18n/{lang}.json", handlers::i18n_handler::i18n_json)
                    .with_post("/login", handlers::auth::login)
                    .with_post("/register", handlers::auth::register)
                    .with_post("/logout", handlers::auth::logout),
            ),
        );

    elog!(Info, "Rama Platform server listening on http://{bind}");

    let service = CompressionLayer::new().layer(app);
    HttpServer::http1()
        .listen(bind, service)
        .await
        .expect("failed to start HTTP server");
}
