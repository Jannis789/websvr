use platform_core::{Config, I18n};
use sea_orm::DatabaseConnection;

// ── Shared State ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SharedState {
    pub config: &'static Config,
    pub db: DatabaseConnection,
    pub i18n: I18n,
    pub server_epoch: u64,
}

impl SharedState {
    pub async fn init() -> Self {
        let config = Config::global();
        let i18n = I18n::new(
            serde_json::from_str(include_str!("../../assets/i18n/de.json"))
                .expect("Failed to parse de.json"),
            serde_json::from_str(include_str!("../../assets/i18n/en.json"))
                .expect("Failed to parse en.json"),
        );
        let db = crate::db::init(&config.database_url).await;
        let server_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            config,
            db,
            i18n,
            server_epoch,
        }
    }
}
