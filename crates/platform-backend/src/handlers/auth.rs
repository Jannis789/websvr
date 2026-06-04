use crate::context::SharedState;
use crate::elog;
use crate::entities::users;
use crate::utils::request::extract_context;
use crate::utils::response::{empty_response, Response};
use platform_core::PasswordUtil;
use rama::http::service::web::extract::State;
use rama::http::{header, Request, StatusCode};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::json;

/// Maximum allowed body size for login/register forms (10 KiB).
const MAX_BODY_SIZE: usize = 10 * 1024;

/// Minimum password length.
const MIN_PASSWORD_LEN: usize = 8;

/// RFC 5322-ish email pattern — compiled once (lazy).
static EMAIL_REGEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}$").unwrap()
});

#[derive(Debug, Deserialize)]
struct LoginPayload {
    email: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegisterPayload {
    username: Option<String>,
    email: Option<String>,
    password: Option<String>,
    confirm_password: Option<String>,
}

/// Read the request body with a size limit.
async fn read_body_limited(req: Request) -> Option<Vec<u8>> {
    let mut body = req.into_body();
    let mut all_bytes = Vec::new();
    loop {
        match body.chunk().await {
            Ok(Some(chunk)) => {
                if all_bytes.len() + chunk.len() > MAX_BODY_SIZE {
                    return None;
                }
                all_bytes.extend_from_slice(&chunk);
            }
            Ok(None) => return Some(all_bytes),
            Err(e) => {
                elog!(Error, "Failed to read request body: {e}");
                return None;
            }
        }
    }
}

/// Push error signals into the SSE broadcaster, return 200 OK.
/// Events arrive through the persistent /sse stream.
fn emit_error(ctx: &crate::context::ClientContext, field: &str, message: impl AsRef<str>) -> Response {
    let msg = message.as_ref();
    let signals = json!({ "errors": { field: msg }, "submitting": false, "success": false });
    let signals_json = serde_json::to_string(&signals).unwrap();
    ctx.event_emitter.emit_signal(&signals_json);
    empty_response(StatusCode::OK)
}

/// Push success signals + redirect script into the broadcaster, return 200 OK.
/// Events arrive through the persistent /sse stream.
fn emit_success(ctx: &crate::context::ClientContext, redirect_url: &str) -> Response {
    // Clear stale events (e.g. login form) before emitting post-login state
    ctx.event_emitter.clear();

    let signals = json!({ "errors": "", "success": true });
    let signals_json = serde_json::to_string(&signals).unwrap();
    ctx.event_emitter.emit_signal(&signals_json);
    ctx.event_emitter.try_emit_script(&format!(
        "setTimeout(() => {{ window.location.href = '{}'; }}, 1200);",
        redirect_url
    ));
    empty_response(StatusCode::OK)
}

/// POST /login — authenticate user via email + password.
/// Datastar @post sends signals as JSON body.
/// Returns 200 OK; the Dreifaltigkeit arrives via the persistent /sse stream.
pub async fn login(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let client_id = ctx.client_id.to_string();

    // Debug: log request metadata to diagnose double-POST
    let ct = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    let cl = req
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    elog!(
        Info,
        "POST /login arrived [client={}, ct={}, cl={}]",
        client_id,
        ct,
        cl
    );

    // Idempotency: if already authenticated, just return OK (prevents double-click issues)
    let already_auth = ctx
        .session_storage
        .try_lock()
        .map(|g| g.get("authenticated").and_then(|v| v.as_bool()).unwrap_or(false))
        .unwrap_or(false);
    if already_auth {
        elog!(
            Info,
            "POST /login skipped (already authenticated) [client={}]",
            client_id
        );
        return empty_response(StatusCode::OK);
    }

    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return emit_error(&ctx, "error", "Request too large"),
    };

    let form: LoginPayload = match serde_json::from_slice(&all_bytes) {
        Ok(f) => f,
        Err(_) => return emit_error(&ctx, "error", "Invalid request"),
    };

    let email = match form.email {
        Some(ref e) if !e.is_empty() => e,
        _ => return emit_error(&ctx, "email", "E-Mail erforderlich"),
    };

    if !EMAIL_REGEX.is_match(email) {
        return emit_error(&ctx, "email", "Ungueltige E-Mail-Adresse");
    }

    let password = match form.password {
        Some(ref p) if !p.is_empty() => p,
        _ => return emit_error(&ctx, "password", "Passwort erforderlich"),
    };

    elog!(
        Info,
        "Login attempt for: {} [client={}, body={} bytes]",
        email,
        client_id,
        all_bytes.len()
    );

    // Query user by email
    let user = match users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return emit_error(&ctx, "email", "Kein Account mit dieser E-Mail");
        }
        Err(e) => {
            elog!(Error, "DB query failed during login: {e}");
            return emit_error(&ctx, "error", "Interner Fehler");
        }
    };

    // Verify password via constant-time comparison
    if !PasswordUtil::verify_password(password, &user.password_hash) {
        return emit_error(&ctx, "password", "Falsches Passwort");
    }

    // Mark session as authenticated
    {
        let mut session = ctx.session_storage.lock().await;
        session.set_volatile("authenticated", serde_json::Value::Bool(true));
        session.set_volatile("user_id", serde_json::Value::Number(user.id.into()));
    }
    elog!(
        Info,
        "Session authenticated for {} (user_id={})",
        ctx.client_id,
        user.id
    );

    emit_success(&ctx, "/home")
}

/// POST /register — create new user account.
/// Returns 200 OK; events arrive via the persistent /sse stream.
pub async fn register(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);

    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return emit_error(&ctx, "error", "Request too large"),
    };

    let form: RegisterPayload = match serde_json::from_slice(&all_bytes) {
        Ok(f) => f,
        Err(_) => return emit_error(&ctx, "error", "Invalid request"),
    };

    let username = match form.username {
        Some(ref u) if !u.is_empty() => u,
        _ => return emit_error(&ctx, "username", "Benutzername erforderlich"),
    };

    let email = match form.email {
        Some(ref e) if !e.is_empty() => e,
        _ => return emit_error(&ctx, "email", "E-Mail erforderlich"),
    };

    if !EMAIL_REGEX.is_match(email) {
        return emit_error(&ctx, "email", "Ungueltige E-Mail-Adresse");
    }

    let password = match form.password {
        Some(ref p) if !p.is_empty() => p,
        _ => return emit_error(&ctx, "password", "Passwort erforderlich"),
    };

    if password.len() < MIN_PASSWORD_LEN {
        return emit_error(
            &ctx,
            "password",
            format!("Mindestens {} Zeichen", MIN_PASSWORD_LEN),
        );
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        return emit_error(&ctx, "password", "Grossbuchstabe erforderlich");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return emit_error(&ctx, "password", "Ziffer erforderlich");
    }

    let confirm = match form.confirm_password {
        Some(ref p) if !p.is_empty() => p,
        _ => return emit_error(&ctx, "confirm", "Passwort bestaetigen"),
    };

    if password != confirm {
        return emit_error(&ctx, "confirm", "Passwoerter stimmen nicht");
    }

    elog!(Info, "Register attempt: {} <{}>", username, email);

    // Check email uniqueness
    match users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {
            return emit_error(&ctx, "email", "Bereits registriert");
        }
        Err(e) => {
            elog!(Error, "DB query failed during register (email check): {e}");
            return emit_error(&ctx, "error", "Interner Fehler");
        }
        _ => {}
    }

    // Check username uniqueness
    match users::Entity::find()
        .filter(users::Column::Username.eq(username))
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {
            return emit_error(&ctx, "username", "Bereits vergeben");
        }
        Err(e) => {
            elog!(Error, "DB query failed during register (username check): {e}");
            return emit_error(&ctx, "error", "Interner Fehler");
        }
        _ => {}
    }

    // Hash password with ring HMAC-SHA256
    let password_hash = PasswordUtil::hash_new(password);

    // Insert new user
    let active = users::ActiveModel {
        username: Set(username.clone()),
        email: Set(email.clone()),
        password_hash: Set(password_hash),
        ..Default::default()
    };

    if let Err(e) = active.insert(&state.db).await {
        elog!(Error, "Failed to insert user during register: {e}");
        return emit_error(&ctx, "error", "Registrierung fehlgeschlagen");
    }

    elog!(Info, "User registered: {} <{}>", username, email);
    emit_success(&ctx, "/login")
}

/// POST /logout — clear session and redirect to login via SSE executeScript.
pub async fn logout(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);

    // Clear session
    {
        let mut session = ctx.session_storage.lock().await;
        session.set_volatile("authenticated", serde_json::Value::Null);
        session.set_volatile("user_id", serde_json::Value::Null);
    }

    // Clear stale events before redirect
    ctx.event_emitter.clear();

    // Push redirect script into broadcaster
    ctx.event_emitter
        .try_emit_script("setTimeout(() => { window.location.href = '/login'; }, 500);");

    let mut resp = empty_response(StatusCode::OK);
    let clear_cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        platform_core::client_id::CLIENT_ID_COOKIE
    );
    resp.headers_mut()
        .insert(header::SET_COOKIE, clear_cookie.parse().unwrap());
    resp
}
