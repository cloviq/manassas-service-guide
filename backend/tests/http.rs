use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

#[tokio::test]
async fn ticket_endpoint_rejects_bad_input_without_exposing_a_read_api() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let app = service_guide::app(pool);
    for (method, uri, body, expected) in [
        (
            "POST",
            "/api/tickets",
            r#"{"description":"short"}"#.to_owned(),
            400,
        ),
        ("POST", "/api/tickets", "x".repeat(20000), 413),
        ("GET", "/api/tickets", String::new(), 405),
        ("GET", "/api/tickets/123", String::new(), 404),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), expected, "{method} {uri}");
    }
}

#[tokio::test]
async fn service_read_fails_safely_when_database_is_unavailable() {
    let pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let response = service_guide::app(pool)
        .oneshot(
            Request::builder()
                .uri("/api/services")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 503);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["error"], "Service temporarily unavailable.");
}
