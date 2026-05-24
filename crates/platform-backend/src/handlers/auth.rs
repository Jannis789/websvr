use crate::elog;
use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use rama::http::header;
use crate::server::SharedState;
use serde::Deserialize;
use crate::utils::request::{extract_context};
use crate::utils::response::{empty_response, redirect, html_response};

/// Maximum allowed body size for login/register forms (10 KiB).
const MAX_BODY_SIZE: usize = 10 * 1024;

#[derive(Debug, Deserialize)]
struct LoginForm {
    email: String,
    #[allow(dead_code)]
    password: String,
}

#[derive(Debug, Deserialize)]
struct RegisterForm {
    username: String,
    email: String,
    password: String,
}

/// Read the request body with a size limit.
/// Returns `None` if the body exceeds MAX_BODY_SIZE.
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

/// POST /login — authenticate user via email + password
pub async fn login(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);

    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return empty_response(StatusCode::PAYLOAD_TOO_LARGE),
    };

    let form: LoginForm = match serde_urlencoded::from_bytes(&all_bytes) {
        Ok(f) => f,
        Err(e) => {
            elog!(Warn, "Invalid login form: {e}");
            return html_response(include_str!("../../assets/templates/login.html"));
        }
    };

    elog!(Info, "Login attempt for: {}", form.email);

    // TODO Phase 2: Query DB via SeaORM, verify password
    // let user = UserEntity::find()
    //     .filter(user::Column::Email.eq(&form.email))
    //     .one(&state.db).await
    //     .expect("DB query failed");
    //
    // match user {
    //     Some(user) if PasswordUtil::verify_password(&form.password, &user.password_hash) => {
    //         // Create session in DB
    //         tracing::info!("Login successful for: {}", form.email);
    //         return redirect("/home");
    //     }
    //     _ => {
    //         tracing::warn!("Login failed for: {}", form.email);
    //     }
    // }
    let _ = state;

    // Placeholder: mark session as authenticated and redirect to home
    {
        let mut session = ctx.session_storage.lock().await;
        session.set_volatile("authenticated", serde_json::Value::Bool(true));
    }
    elog!(Info, "Session → authenticated for {}", ctx.client_id);
    redirect("/home")
}

/// POST /register — create new user account
pub async fn register(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return empty_response(StatusCode::PAYLOAD_TOO_LARGE),
    };

    let form: RegisterForm = match serde_urlencoded::from_bytes(&all_bytes) {
        Ok(f) => f,
        Err(e) => {
            elog!(Warn, "Invalid register form: {e}");
            return html_response(include_str!("../../assets/templates/register.html"));
        }
    };

    elog!(Info, "Register attempt: {} <{}>", form.username, form.email);

    // Validate password length
    if form.password.len() < 8 {
        elog!(Warn, "Password too short for: {}", form.email);
        return html_response(include_str!("../../assets/templates/register.html"));
    }

    // TODO Phase 2: Check uniqueness, hash password, insert user via SeaORM
    let _ = state;

    elog!(Info, "Registration placeholder — redirecting to login");
    redirect("/login")
}

/// POST /logout — clear session
pub async fn logout(
    State(_state): State<SharedState>,
    _req: Request,
) -> Response {
    // Clear client_id cookie by setting Max-Age=0
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