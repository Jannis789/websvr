use migration::Migrator;
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    // Load .env before migration CLI reads DATABASE_URL
    dotenvy::dotenv().ok();

    // Fallback: set DATABASE_URL if still missing after .env load
    if std::env::var("DATABASE_URL").is_err() {
        std::env::set_var("DATABASE_URL", "sqlite://platform.db?mode=rwc");
    }

    cli::run_cli(Migrator).await;
}
