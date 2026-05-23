use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use rama::http::header;
use crate::server::SharedState;
use crate::common::{self, redirect, html_response, empty_response};
use serde::Deserialize;

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

/// POST /login — authenticate user via email + password
pub async fn login(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    // Extract ClientId from request extensions (set by AuthService layer)
    let _cid = common::extract_context(&req).client_id;

    // Collect request body chunks
    let mut body = req.into_body();
    let mut all_bytes = Vec::new();
    loop {
        match body.chunk().await {
            Ok(Some(chunk)) => all_bytes.extend_from_slice(&chunk),
            Ok(None) => break,
            Err(e) => {
                tracing::error!("Failed to read login request body: {e}");
                return empty_response(StatusCode::BAD_REQUEST);
            }
        }
    }

    let form: LoginForm = match serde_urlencoded::from_bytes(&all_bytes) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Invalid login form: {e}");
            return html_response(include_str!("../../assets/templates/login.html"));
        }
    };

    tracing::info!("Login attempt for: {}", form.email);

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
    //         // Return login page with error (Datastar signal for inline error)
    //     }
    // }
    let _ = state; // Placeholder: suppress unused warning until Phase 2

    // Placeholder: redirect to home on any login attempt
    redirect("/home")
}

/// POST /register — create new user account
pub async fn register(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    // Collect request body chunks
    let mut body = req.into_body();
    let mut all_bytes = Vec::new();
    loop {
        match body.chunk().await {
            Ok(Some(chunk)) => all_bytes.extend_from_slice(&chunk),
            Ok(None) => break,
            Err(e) => {
                tracing::error!("Failed to read register request body: {e}");
                return empty_response(StatusCode::BAD_REQUEST);
            }
        }
    }

    let form: RegisterForm = match serde_urlencoded::from_bytes(&all_bytes) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Invalid register form: {e}");
            return html_response(include_str!("../../assets/templates/register.html"));
        }
    };

    tracing::info!("Register attempt: {} <{}>", form.username, form.email);

    // Validate password length
    if form.password.len() < 8 {
        tracing::warn!("Password too short for: {}", form.email);
        return html_response(include_str!("../../assets/templates/register.html"));
    }

    // TODO Phase 2: Check uniqueness, hash password, insert user via SeaORM
    // let password_hash = PasswordUtil::hash_new(&form.password);
    // let user = user::ActiveModel {
    //     username: Set(form.username),
    //     email: Set(form.email),
    //     password_hash: Set(password_hash),
    //     ..Default::default()
    // };
    // user.insert(&state.db).await.expect("Failed to insert user");
    let _ = state;

    tracing::info!("Registration placeholder — redirecting to login");
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
