use std::sync::OnceLock;

/// Singleton application configuration, loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub rust_log: String,
    pub client_id_ttl_days: u32,
    pub sse_ttl_days: u32,
    pub hmac_secret: String,
}

impl Config {
    /// Retrieve the global singleton Config.
    /// Initialises on first call from environment variables.
    pub fn global() -> &'static Config {
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(|| Config::from_env())
    }

    fn from_env() -> Config {
        let hmac_secret = std::env::var("HMAC_SECRET")
            .unwrap_or_else(|_| {
                #[cfg(debug_assertions)]
                {
                    eprintln!("[WARN] HMAC_SECRET not set — using insecure default. Set HMAC_SECRET env var for production.");
                    "default-dev-secret-change-in-production".to_string()
                }
                #[cfg(not(debug_assertions))]
                {
                    panic!("HMAC_SECRET environment variable must be set in production builds");
                }
            });

        Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://platform.db?mode=rwc".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            rust_log: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            client_id_ttl_days: std::env::var("CLIENT_ID_TTL_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            sse_ttl_days: std::env::var("SSE_TTL_DAYS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            hmac_secret,
        }
    }
}
