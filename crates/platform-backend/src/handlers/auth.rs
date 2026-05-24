use crate::elog;
use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use rama::http::header;
use crate::server::SharedState;
use serde::Deserialize;
use crate::utils::request::extract_context;
use crate::utils::response::{empty_response, redirect, html_response};

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

/// Build an SSE response that merges the `error` signal.
/// Datastar processes this as a single SSE event.
fn error_sse(message: &str) -> Response {
    let body = format!(
        "event: datastar-merge-signals\ndata: {{\"error\":\"{}\"}}\n\n",
        message.replace('"', "\\\"")
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(body.into())
        .unwrap()
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
        None => return error_sse("Request too large"),
    };

    let form: LoginPayload = match serde_json::from_slice(&all_bytes) {
        Ok(f) => f,
        Err(_) => return error_sse("Invalid request"),
    };

    let email = match form.email {
        Some(ref e) if !e.is_empty() => e,
        _ => return error_sse("Email is required"),
    };

    if form.password.as_ref().map_or(true, |p| p.is_empty()) {
        return error_sse("Password is required");
    }

    elog!(Info, "Login attempt for: {}", email);

    // TODO Phase 2: Query DB via SeaORM, verify password
    let _ = state;

    // Placeholder: mark session as authenticated
    {
        let mut session = ctx.session_storage.lock().await;
        session.set_volatile("authenticated", serde_json::Value::Bool(true));
    }
    elog!(Info, "Session → authenticated for {}", ctx.client_id);

    // Success: redirect to /home
    let mut resp = redirect("/home");
    // Tell Datastar this is a redirect it should follow
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/event-stream".parse().unwrap(),
    );
    resp
}

/// POST /register — create new user account
pub async fn register(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    let all_bytes = match read_body_limited(req).await {
        Some(b) => b,
        None => return error_sse("Request too large"),
    };

    let form: RegisterPayload = match serde_json::from_slice(&all_bytes) {
        Ok(f) => f,
        Err(_) => return error_sse("Invalid request"),
    };

    let username = match form.username {
        Some(ref u) if !u.is_empty() => u,
        _ => return error_sse("Username is required"),
    };

    let email = match form.email {
        Some(ref e) if !e.is_empty() => e,
        _ => return error_sse("Email is required"),
    };

    let password = match form.password {
        Some(ref p) if p.len() >= 8 => p,
        Some(ref p) if !p.is_empty() => return error_sse("Password must be at least 8 characters"),
        _ => return error_sse("Password is required"),
    };

    elog!(Info, "Register attempt: {} <{}>", username, email);

    // TODO Phase 2: Check uniqueness, hash password, insert user via SeaORM
    let _ = (state, password);

    elog!(Info, "Registration placeholder — redirecting to login");
    redirect("/login")
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
