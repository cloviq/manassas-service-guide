//! Accounts, cookie sessions and the admin user API. Every route except login,
//! logout and /healthz requires a session; /api/admin/* additionally requires admin.
use crate::api::ApiError;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Extension, Json, Router,
    extract::{Path, Request, State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::LazyLock;
use uuid::Uuid;

const COOKIE: &str = "session";
const SESSION_DAYS: i32 = 7;
const USER_COLUMNS: &str = "id,username,role,to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at";
// Verified when the username is unknown so response time does not reveal accounts.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_blocking("not-a-real-account-password").expect("dummy hash"));

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: String,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: String,
}
#[derive(Deserialize)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}
#[derive(Deserialize)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub role: String,
}
#[derive(Deserialize)]
pub struct UserChange {
    pub role: Option<String>,
    pub password: Option<String>,
}

pub fn normalize_username(raw: &str) -> Result<String, &'static str> {
    let name = raw.trim().to_lowercase();
    if (3..=64).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
    {
        Ok(name)
    } else {
        Err("Username must be 3–64 characters: letters, digits, '.', '_' or '-'.")
    }
}
pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if (12..=256).contains(&password.chars().count()) {
        Ok(())
    } else {
        Err("Password must be 12–256 characters.")
    }
}
pub fn validate_role(role: &str) -> Result<(), &'static str> {
    if role == "user" || role == "admin" {
        Ok(())
    } else {
        Err("Role must be user or admin.")
    }
}
fn bad(message: &'static str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message)
}
fn json<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    payload.map(|Json(value)| value).map_err(|rejection| {
        ApiError(
            rejection.status(),
            "Invalid JSON or request body too large.",
        )
    })
}

fn hash_blocking(password: &str) -> Option<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .ok()
        .map(|hash| hash.to_string())
}
async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || hash_blocking(&password))
        .await
        .ok()
        .flatten()
        .ok_or(ApiError(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not store password.",
        ))
}
async fn verify_password(password: String, hash: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || {
        let hash = hash.unwrap_or_else(|| DUMMY_HASH.clone());
        PasswordHash::new(&hash).is_ok_and(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
    })
    .await
    .unwrap_or(false)
}

fn new_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().strip_prefix("session="))
        .find(|token| token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_owned)
}
fn session_cookie(token: &str, max_age: i32) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{COOKIE}={token}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={max_age}"
    ))
    .expect("cookie header")
}
async fn current_user(pool: &PgPool, headers: &HeaderMap) -> Result<Option<User>, ApiError> {
    let Some(token) = session_token(headers) else {
        return Ok(None);
    };
    Ok(sqlx::query_as::<_, User>(
        "SELECT u.id,u.username,u.role FROM sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=sha256($1) AND s.expires_at>now()",
    )
    .bind(token.as_bytes())
    .fetch_optional(pool)
    .await?)
}

async fn require_user(
    State(pool): State<PgPool>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let user = current_user(&pool, request.headers())
        .await?
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "Sign in required."))?;
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}
async fn require_admin(request: Request, next: Next) -> Result<Response, ApiError> {
    match request.extensions().get::<User>() {
        Some(user) if user.role == "admin" => Ok(next.run(request).await),
        _ => Err(ApiError(
            StatusCode::FORBIDDEN,
            "Administrator access required.",
        )),
    }
}

async fn login(
    State(pool): State<PgPool>,
    payload: Result<Json<LoginInput>, JsonRejection>,
) -> Result<Response, ApiError> {
    let input = json(payload)?;
    let row: Option<(Uuid, String, String, String)> = match normalize_username(&input.username) {
        Ok(name) if input.password.len() <= 1024 => {
            sqlx::query_as("SELECT id,username,role,password_hash FROM users WHERE username=$1")
                .bind(name)
                .fetch_optional(&pool)
                .await?
        }
        _ => None,
    };
    let hash = row.as_ref().map(|r| r.3.clone());
    let valid = verify_password(input.password, hash).await;
    let Some((id, username, role, _)) = row.filter(|_| valid) else {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "Invalid username or password.",
        ));
    };
    sqlx::query("DELETE FROM sessions WHERE expires_at<now()")
        .execute(&pool)
        .await?;
    let token = new_token();
    sqlx::query("INSERT INTO sessions (token_hash,user_id,expires_at) VALUES (sha256($1),$2,now()+make_interval(days=>$3))")
        .bind(token.as_bytes())
        .bind(id)
        .bind(SESSION_DAYS)
        .execute(&pool)
        .await?;
    Ok((
        [(
            header::SET_COOKIE,
            session_cookie(&token, SESSION_DAYS * 86400),
        )],
        Json(User { id, username, role }),
    )
        .into_response())
}
async fn logout(State(pool): State<PgPool>, headers: HeaderMap) -> Result<Response, ApiError> {
    if let Some(token) = session_token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash=sha256($1)")
            .bind(token.as_bytes())
            .execute(&pool)
            .await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, session_cookie("", 0))],
    )
        .into_response())
}
async fn me(Extension(user): Extension<User>) -> Json<User> {
    Json(user)
}

/// Shared by the admin API and the `create-user` CLI bootstrap.
pub async fn create_user(pool: &PgPool, input: NewUser) -> Result<Account, ApiError> {
    let username = normalize_username(&input.username).map_err(bad)?;
    validate_password(&input.password).map_err(bad)?;
    validate_role(&input.role).map_err(bad)?;
    let hash = hash_password(input.password).await?;
    sqlx::query_as::<_, Account>(&format!(
        "INSERT INTO users (id,username,password_hash,role) VALUES ($1,$2,$3,$4) RETURNING {USER_COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(username)
    .bind(hash)
    .bind(input.role)
    .fetch_one(pool)
    .await
    .map_err(|error| {
        if error.as_database_error().is_some_and(|e| e.is_unique_violation()) {
            ApiError(StatusCode::CONFLICT, "Username already exists.")
        } else {
            error.into()
        }
    })
}
async fn list_users(State(pool): State<PgPool>) -> Result<Json<Vec<Account>>, ApiError> {
    Ok(Json(
        sqlx::query_as(&format!(
            "SELECT {USER_COLUMNS} FROM users ORDER BY username"
        ))
        .fetch_all(&pool)
        .await?,
    ))
}
async fn add_user(
    State(pool): State<PgPool>,
    payload: Result<Json<NewUser>, JsonRejection>,
) -> Result<(StatusCode, Json<Account>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(create_user(&pool, json(payload)?).await?),
    ))
}
fn user_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| bad("Invalid user ID."))
}
async fn update_user(
    State(pool): State<PgPool>,
    Extension(admin): Extension<User>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<UserChange>, JsonRejection>,
) -> Result<Json<Account>, ApiError> {
    let id = user_id(&id)?;
    let change = json(payload)?;
    if change.role.is_none() && change.password.is_none() {
        return Err(bad("Nothing to change."));
    }
    // Admins cannot demote themselves, so at least one admin always remains.
    if let Some(role) = &change.role {
        validate_role(role).map_err(bad)?;
        if id == admin.id && role != "admin" {
            return Err(bad("You cannot remove your own admin role."));
        }
    }
    let hash = match change.password {
        Some(password) => {
            validate_password(&password).map_err(bad)?;
            Some(hash_password(password).await?)
        }
        None => None,
    };
    let mut tx = pool.begin().await?;
    let account = sqlx::query_as::<_, Account>(&format!(
        "UPDATE users SET role=coalesce($2,role),password_hash=coalesce($3,password_hash) WHERE id=$1 RETURNING {USER_COLUMNS}"
    ))
    .bind(id)
    .bind(change.role)
    .bind(&hash)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError(StatusCode::NOT_FOUND, "User not found."))?;
    if hash.is_some() {
        // A reset signs the user out everywhere except the admin's own current session.
        let token = session_token(&headers).unwrap_or_default();
        sqlx::query("DELETE FROM sessions WHERE user_id=$1 AND token_hash<>sha256($2)")
            .bind(id)
            .bind(token.as_bytes())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(account))
}
async fn delete_user(
    State(pool): State<PgPool>,
    Extension(admin): Extension<User>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = user_id(&id)?;
    if id == admin.id {
        return Err(bad("You cannot delete your own account."));
    }
    let deleted = sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "User not found."));
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn health(State(pool): State<PgPool>) -> Result<&'static str, ApiError> {
    sqlx::query("SELECT 1").execute(&pool).await?;
    Ok("ok")
}

/// Wraps the content API: everything in `app` requires a signed-in user.
pub fn with_auth(pool: PgPool, app: Router) -> Router {
    let small_body = axum::extract::DefaultBodyLimit::max(4096);
    let admin = Router::new()
        .route("/api/admin/users", get(list_users).post(add_user))
        .route(
            "/api/admin/users/{id}",
            patch(update_user).delete(delete_user),
        )
        .route_layer(middleware::from_fn(require_admin))
        .layer(small_body)
        .with_state(pool.clone());
    let signed_in = Router::new()
        .route("/api/auth/me", get(me))
        .merge(admin)
        .merge(app)
        .route_layer(middleware::from_fn_with_state(pool.clone(), require_user));
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .layer(small_body)
        .route("/healthz", get(health))
        .with_state(pool)
        .merge(signed_in)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn usernames_are_normalized_and_bounded() {
        assert_eq!(normalize_username("  Some.User ").unwrap(), "some.user");
        for bad in ["ab", "has space", "émile", "a;b", &"x".repeat(65)] {
            assert!(normalize_username(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn passwords_and_roles_are_validated() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("twelve-chars").is_ok());
        assert!(validate_role("admin").is_ok() && validate_role("user").is_ok());
        assert!(validate_role("root").is_err());
    }
    #[test]
    fn hashes_verify_only_the_original_password() {
        let hash = hash_blocking("correct horse battery").unwrap();
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password(b"correct horse battery", &parsed)
                .is_ok()
        );
        assert!(
            Argon2::default()
                .verify_password(b"wrong", &parsed)
                .is_err()
        );
    }
    #[test]
    fn session_cookie_is_parsed_strictly() {
        let token = new_token();
        assert_eq!(token.len(), 64);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("a=b; session={token}").parse().unwrap(),
        );
        assert_eq!(session_token(&headers), Some(token));
        headers.insert(header::COOKIE, "session=../../etc".parse().unwrap());
        assert_eq!(session_token(&headers), None);
        let cookie = session_cookie("x", 60);
        let cookie = cookie.to_str().unwrap();
        for attr in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
            assert!(cookie.contains(attr), "{attr}");
        }
    }
}
