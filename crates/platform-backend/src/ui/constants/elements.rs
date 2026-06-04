// ── HTML Templates ────────────────────────────────────────
// `static` fuer include_str! (einmal im Binary eingebettet).
// Benennung: {SEITE}_{KOMPONENTE} fuer klare Herkunft.

// App Shell
pub static SHELL: &str = include_str!("../../../assets/fragments/shell.html");

// Auth Pages
pub static AUTH_HEADER: &str = include_str!("../../../assets/fragments/auth/header.html");
pub static AUTH_LOGIN_FORM: &str = include_str!("../../../assets/fragments/auth/login-form.html");
pub static AUTH_REGISTER_FORM: &str = include_str!("../../../assets/fragments/auth/register-form.html");

// Home Shell
pub static HOME_SIDEBAR: &str = include_str!("../../../assets/fragments/sidebar/sidebar.html");
pub static HOME_HEADER: &str = include_str!("../../../assets/fragments/main/header.html");
pub static HOME_CONTENT_OVERVIEW: &str = include_str!("../../../assets/fragments/content/overview.html");
pub static HOME_CONTENT_MOVIES: &str = include_str!("../../../assets/fragments/content/movies.html");
pub static HOME_CONTENT_SERIES: &str = include_str!("../../../assets/fragments/content/series.html");

// Settings
pub static SETTINGS_SIDEBAR: &str = include_str!("../../../assets/fragments/settings/sidebar.html");
pub static SETTINGS_HEADER: &str = include_str!("../../../assets/fragments/settings/header.html");
pub static SETTINGS_ACCOUNT: &str = include_str!("../../../assets/fragments/settings/account.html");
