//! Database initialisation via SeaORM.

use sea_orm::DatabaseConnection;

/// Connect to the database. Panics on failure (server cannot start without DB).
pub async fn init(database_url: &str) -> DatabaseConnection {
    sea_orm::Database::connect(database_url)
        .await
        .expect("Failed to connect to database")
}
