use axum::{
    Router,
    body::Body,
    http::{Request, Response},
};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

fn app(pool: sqlx::PgPool) -> Router {
    service_guide::auth::with_auth(pool.clone(), service_guide::app(pool))
}
fn offline_app() -> Router {
    app(PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap())
}
async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: &str,
) -> Response<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap()
}
async fn json(response: Response<Body>) -> serde_json::Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn every_content_and_admin_route_requires_a_session() {
    let app = offline_app();
    for (method, uri) in [
        ("GET", "/api/services"),
        ("GET", "/api/services/20000000-0000-4000-8000-000000000004"),
        ("GET", "/api/map/features"),
        ("POST", "/api/tickets"),
        ("GET", "/api/auth/me"),
        ("GET", "/api/admin/users"),
        ("POST", "/api/admin/users"),
        (
            "PATCH",
            "/api/admin/users/20000000-0000-4000-8000-000000000004",
        ),
        (
            "DELETE",
            "/api/admin/users/20000000-0000-4000-8000-000000000004",
        ),
    ] {
        let response = call(&app, method, uri, None, "{}").await;
        assert_eq!(response.status(), 401, "{method} {uri}");
        assert_eq!(json(response).await["error"], "Sign in required.");
    }
}

#[tokio::test]
async fn sessions_fail_closed_and_logout_clears_cookie() {
    let app = offline_app();
    // A well-formed cookie that cannot be checked (database down) is never accepted.
    let cookie = format!("session={}", "a".repeat(64));
    assert_eq!(
        call(&app, "GET", "/api/services", Some(&cookie), "")
            .await
            .status(),
        503
    );
    assert_eq!(
        call(&app, "POST", "/api/auth/login", None, "{bad")
            .await
            .status(),
        400
    );
    let response = call(&app, "POST", "/api/auth/logout", None, "").await;
    assert_eq!(response.status(), 204);
    let cleared = response.headers()["set-cookie"].to_str().unwrap();
    assert!(cleared.starts_with("session=;") && cleared.contains("Max-Age=0"));
    assert_eq!(call(&app, "GET", "/healthz", None, "").await.status(), 503);
}

/// Live: DATABASE_URL=... cargo test --test auth_http -- --ignored
/// Creates uniquely named accounts and deletes them afterwards.
#[tokio::test]
#[ignore]
async fn live_login_roles_and_admin_user_management() {
    let pool = PgPoolOptions::new()
        .connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    let app = app(pool.clone());
    let tag = &uuid::Uuid::new_v4().simple().to_string()[..8];
    let (admin_name, user_name) = (format!("t-admin-{tag}"), format!("t-user-{tag}"));
    let admin = service_guide::auth::create_user(
        &pool,
        service_guide::auth::NewUser {
            username: admin_name.clone(),
            password: "admin-password-1".into(),
            role: "admin".into(),
        },
    )
    .await
    .unwrap();
    let login = |name: String, password: &'static str| {
        let app = app.clone();
        async move {
            let body = serde_json::json!({"username": name, "password": password}).to_string();
            call(&app, "POST", "/api/auth/login", None, &body).await
        }
    };
    let cookie_of = |response: &Response<Body>| {
        response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
    };

    assert_eq!(
        login(admin_name.clone(), "wrong-password!").await.status(),
        401
    );
    assert_eq!(
        login("no-such-user".into(), "admin-password-1")
            .await
            .status(),
        401
    );
    let response = login(admin_name.to_uppercase(), "admin-password-1").await;
    assert_eq!(response.status(), 200);
    let admin_cookie = cookie_of(&response);
    let me = json(call(&app, "GET", "/api/auth/me", Some(&admin_cookie), "").await).await;
    assert_eq!(
        (me["username"].as_str(), me["role"].as_str()),
        (Some(admin_name.as_str()), Some("admin"))
    );
    assert_eq!(
        call(
            &app,
            "GET",
            "/api/services?limit=1",
            Some(&admin_cookie),
            ""
        )
        .await
        .status(),
        200
    );

    let new_user =
        serde_json::json!({"username": user_name, "password": "user-password-1", "role": "user"})
            .to_string();
    let created = call(
        &app,
        "POST",
        "/api/admin/users",
        Some(&admin_cookie),
        &new_user,
    )
    .await;
    assert_eq!(created.status(), 201);
    let user_id = json(created).await["id"].as_str().unwrap().to_owned();
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/admin/users",
            Some(&admin_cookie),
            &new_user
        )
        .await
        .status(),
        409
    );
    let weak = serde_json::json!({"username": "weak-user", "password": "short", "role": "user"})
        .to_string();
    assert_eq!(
        call(&app, "POST", "/api/admin/users", Some(&admin_cookie), &weak)
            .await
            .status(),
        400
    );

    // Regular users can use the guide but not the admin API.
    let user_cookie = cookie_of(&login(user_name.clone(), "user-password-1").await);
    assert_eq!(
        call(&app, "GET", "/api/services?limit=1", Some(&user_cookie), "")
            .await
            .status(),
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/admin/users", Some(&user_cookie), "")
            .await
            .status(),
        403
    );

    // Promotion takes effect on the existing session; self-demotion/deletion is refused.
    let user_uri = format!("/api/admin/users/{user_id}");
    let admin_uri = format!("/api/admin/users/{}", admin.id);
    assert_eq!(
        call(
            &app,
            "PATCH",
            &user_uri,
            Some(&admin_cookie),
            r#"{"role":"admin"}"#
        )
        .await
        .status(),
        200
    );
    assert_eq!(
        call(&app, "GET", "/api/admin/users", Some(&user_cookie), "")
            .await
            .status(),
        200
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &admin_uri,
            Some(&admin_cookie),
            r#"{"role":"user"}"#
        )
        .await
        .status(),
        400
    );
    assert_eq!(
        call(&app, "DELETE", &admin_uri, Some(&admin_cookie), "")
            .await
            .status(),
        400
    );

    // Password reset revokes that user's sessions.
    let reset = call(
        &app,
        "PATCH",
        &user_uri,
        Some(&admin_cookie),
        r#"{"password":"new-password-22"}"#,
    )
    .await;
    assert_eq!(reset.status(), 200);
    assert_eq!(
        call(&app, "GET", "/api/auth/me", Some(&user_cookie), "")
            .await
            .status(),
        401
    );
    assert_eq!(
        login(user_name.clone(), "user-password-1").await.status(),
        401
    );
    assert_eq!(
        login(user_name.clone(), "new-password-22").await.status(),
        200
    );

    assert_eq!(
        call(&app, "DELETE", &user_uri, Some(&admin_cookie), "")
            .await
            .status(),
        204
    );
    assert_eq!(login(user_name, "new-password-22").await.status(), 401);
    assert_eq!(
        call(&app, "POST", "/api/auth/logout", Some(&admin_cookie), "")
            .await
            .status(),
        204
    );
    assert_eq!(
        call(&app, "GET", "/api/auth/me", Some(&admin_cookie), "")
            .await
            .status(),
        401
    );
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(admin.id)
        .execute(&pool)
        .await
        .unwrap();
}
