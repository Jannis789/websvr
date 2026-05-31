use crate::components::sidebar::Sidebar;
use crate::components::Shell;
use crate::context::SharedState;
use crate::elog;
use crate::entities::users;
use crate::utils::request::extract_context;
use crate::utils::response::{html_response, Response};
use rama::http::service::web::extract::State;
use rama::http::Request;
use sea_orm::EntityTrait;

static SHELL: &str = include_str!("../../../assets/fragments/shell.html");

static SETTINGS_SIDEBAR: &str = include_str!("../../../assets/fragments/settings/sidebar.html");
static SETTINGS_HEADER: &str = include_str!("../../../assets/fragments/settings/header.html");
static SETTINGS_ACCOUNT: &str = include_str!("../../../assets/fragments/settings/account.html");

const I18N_KEYS_SETTINGS: &[&str] = &[
    "app_name", "aria_menu", "app_brand", "settings_title",
    "settings_account", "settings_account_subtitle",
    "settings_username", "settings_email",
    "settings_new_password", "settings_confirm_password",
    "settings_password_placeholder", "settings_logout",
];

const I18N_KEYS_ACCOUNT: &[&str] = &[
    "settings_account", "settings_account_subtitle",
    "settings_username", "settings_email",
    "settings_new_password", "settings_confirm_password",
    "settings_password_placeholder",
];

/// GET /settings — settings page shell
pub async fn settings_page(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);

    let (username, email) = load_user_from_session(&state, &ctx).await;

    elog!(Debug, "Handler → settings_page (client_id={})", ctx.client_id);

    let mut signals: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&state.i18n.resolve_signals(ctx.lang, I18N_KEYS_SETTINGS))
            .unwrap();
    signals.insert("settingsPage".into(), serde_json::Value::String("account".into()));
    signals.insert("username".into(), serde_json::Value::String(username));
    signals.insert("email".into(), serde_json::Value::String(email));

    Shell::empty()
        .add(Sidebar::full(SETTINGS_SIDEBAR))
        .header(SETTINGS_HEADER)
        .content(SETTINGS_ACCOUNT)
        .signals(&serde_json::to_string(&signals).unwrap())
        .emit(&ctx);

    html_response(SHELL)
}

/// GET /settings/account — swap content slot only
pub async fn get_settings_account(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);

    let (username, email) = load_user_from_session(&state, &ctx).await;

    let mut signals: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&state.i18n.resolve_signals(ctx.lang, I18N_KEYS_ACCOUNT))
            .unwrap();
    signals.insert("settingsPage".into(), serde_json::Value::String("account".into()));
    signals.insert("username".into(), serde_json::Value::String(username));
    signals.insert("email".into(), serde_json::Value::String(email));

    Shell::empty()
        .content(SETTINGS_ACCOUNT)
        .signals(&serde_json::to_string(&signals).unwrap())
        .emit_response(&ctx)
}

/// Load username + email from DB using user_id stored in session.
async fn load_user_from_session(
    state: &SharedState,
    ctx: &crate::context::ClientContext,
) -> (String, String) {
    let session = ctx.session_storage.lock().await;
    let user_id: i32 = match session.get("user_id").and_then(|v| v.as_i64()) {
        Some(id) => id as i32,
        None => {
            drop(session);
            return ("Unbekannt".to_string(), String::new());
        }
    };
    drop(session);

    match users::Entity::find_by_id(user_id).one(&state.db).await {
        Ok(Some(user)) => (user.username, user.email),
        Ok(None) => ("Unbekannt".to_string(), String::new()),
        Err(e) => {
            elog!(Error, "DB query failed loading user: {e}");
            ("Fehler".to_string(), String::new())
        }
    }
}
