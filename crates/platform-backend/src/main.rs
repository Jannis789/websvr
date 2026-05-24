use platform_backend::server;
use platform_backend::elog;

#[tokio::main]
async fn main() {
    // Load .env file (silently ignores if file is missing)
    dotenvy::dotenv().ok();

    elog!(Info, "Starting Rama Platform server");
    server::run().await;
}
