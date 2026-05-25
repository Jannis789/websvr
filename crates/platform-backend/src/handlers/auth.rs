use crate::elog;
use rama::http::{Request, Response, header};
use rama::http::service::web::extract::State;
use crate::server::SharedState;
use serde::Deserialize;
use serde_json::json;
use crate::utils::request::extract_context;
use crate::utils::response::redirect;
use crate::context::{ClientContextSseExt, sse_response};
use crate::entities::users;
use platform_core::PasswordUtil;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, ActiveModelTrait, Set};

/// Maximum allowed body size for login/register forms (10 KiB).
const MAX_BODY_SIZE: usize = 10 * 1024;

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

/// Emit a `datastar-patch-signals` event via the event emitter and return
/// the SSE response for Datastar's `@post` handler.
fn emit_signals_json(ctx: &crate::client_context::ClientContext, value: serde_json::Value) -> Response {
    let signals_json = serde_json::to_string(&value).unwrap();
    let event = ctx.emit_signals(&signals_json);
    sse_response(&event)
}

/// POST /login — authenticate user via email + password
/// Datastar @post sends signals as JSON body.
pub async fn login(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);

    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return emit_signals_json(&ctx, json!({"error": "Request too large"})),
    };

    let form: LoginPayload = match serde_json::from_slice(&all_bytes) {
        Ok(f) => f,
        Err(_) => return emit_signals_json(&ctx, json!({"error": "Invalid request"})),
    };

    let email = match form.email {
        Some(ref e) if !e.is_empty() => e,
        _ => return emit_signals_json(&ctx, json!({"error": "Email is required"})),
    };

    if form.password.as_ref().map_or(true, |p| p.is_empty()) {
        return emit_signals_json(&ctx, json!({"error": "Password is required"}));
    }

    elog!(Info, "Login attempt for: {}", email);

    // Query user by email
    let user = match users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(&state.db)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return emit_signals_json(&ctx, json!({"error": "Invalid email or password"}));
        }
        Err(e) => {
            elog!(Error, "DB query failed during login: {e}");
            return emit_signals_json(&ctx, json!({"error": "Internal server error"}));
        }
    };

    // Verify password via constant-time comparison
    if !PasswordUtil::verify_password(form.password.as_deref().unwrap(), &user.password_hash) {
        return emit_signals_json(&ctx, json!({"error": "Invalid email or password"}));
    }

    // Mark session as authenticated
    {
        let mut session = ctx.session_storage.lock().await;
        session.set_volatile("authenticated", serde_json::Value::Bool(true));
    }
    elog!(Info, "Session → authenticated for {} (user_id={})", ctx.client_id, user.id);

    emit_signals_json(&ctx, json!({
        "error": "",
        "message": "Login successful — redirecting…",
        "redirect": "/home",
    }))
}

/// POST /register — create new user account
pub async fn register(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);

    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return emit_signals_json(&ctx, json!({"error": "Request too large"})),
    };

    let form: RegisterPayload = match serde_json::from_slice(&all_bytes) {
        Ok(f) => f,
        Err(_) => return emit_signals_json(&ctx, json!({"error": "Invalid request"})),
    };

    let username = match form.username {
        Some(ref u) if !u.is_empty() => u,
        _ => return emit_signals_json(&ctx, json!({"error": "Username is required"})),
    };

    let email = match form.email {
        Some(ref e) if !e.is_empty() => e,
        _ => return emit_signals_json(&ctx, json!({"error": "Email is required"})),
    };

    let password = match form.password {
        Some(ref p) if p.len() >= 8 => p,
        Some(ref p) if !p.is_empty() => return emit_signals_json(&ctx, json!({"error": "Password must be at least 8 characters"})),
        _ => return emit_signals_json(&ctx, json!({"error": "Password is required"})),
    };

    elog!(Info, "Register attempt: {} <{}>", username, email);

    // Check email uniqueness
    match users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {
            return emit_signals_json(&ctx, json!({"error": "Email already registered"}));
        }
        Err(e) => {
            elog!(Error, "DB query failed during register (email check): {e}");
            return emit_signals_json(&ctx, json!({"error": "Internal server error"}));
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
            return emit_signals_json(&ctx, json!({"error": "Username already taken"}));
        }
        Err(e) => {
            elog!(Error, "DB query failed during register (username check): {e}");
            return emit_signals_json(&ctx, json!({"error": "Internal server error"}));
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
        return emit_signals_json(&ctx, json!({"error": "Registration failed — please try again"}));
    }

    elog!(Info, "User registered: {} <{}>", username, email);
    emit_signals_json(&ctx, json!({
        "error": "",
        "message": "Registration successful — redirecting to login…",
        "redirect": "/login",
    }))
}

/// POST /logout — clear session and redirect to login
pub async fn logout(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    let mut resp = redirect("/login");
    let clear_cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        platform_core::client_id::CLIENT_ID_COOKIE
    );
    resp.headers_mut().insert(
        header::SET_COOKIE,
        clear_cookie.parse().unwrap(),
    );
    resp
}
