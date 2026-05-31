use platform_backend::elog;
use platform_backend::routes;

#[tokio::main]
async fn main() {
    // Load .env file (silently ignores if file is missing)
    dotenvy::dotenv().ok();

    elog!(Info, "Starting Rama Platform server");
    routes::run().await;
}
