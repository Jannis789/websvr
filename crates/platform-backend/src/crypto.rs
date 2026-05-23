use platform_core::Config;

/// Return the HMAC secret from the global config.
pub fn hmac_secret() -> String {
    Config::global().hmac_secret.clone()
}
