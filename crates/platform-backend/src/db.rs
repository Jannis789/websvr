//! Database initialisation via SeaORM.

use sea_orm::DatabaseConnection;
use sea_orm_migration::MigratorTrait;

/// Connect to the database and run pending migrations.
/// Panics on failure (server cannot start without DB).
pub async fn init(database_url: &str) -> DatabaseConnection {
    let db = sea_orm::Database::connect(database_url)
        .await
        .expect("Failed to connect to database");

    // Run pending migrations (auto-creates tables)
    migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run database migrations");

    db
}
