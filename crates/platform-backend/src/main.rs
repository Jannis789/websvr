use platform_backend::server;

#[tokio::main]
async fn main() {
    // Load .env file (silently ignores if file is missing)
    dotenvy::dotenv().ok();

    // Initialise structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    tracing::info!("Starting Rama Platform server");
    server::run().await;
}
