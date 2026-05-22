use migration::Migrator;
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
    // Load .env before migration CLI reads DATABASE_URL
    dotenvy::dotenv().ok();
    cli::run_cli(Migrator).await;
}
