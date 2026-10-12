use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use tower::ServiceExt;

// Uses a local HTTP fixture only to exercise the OpenSearch transport branch.
// Real containers are covered separately by tests/live.rs.
#[tokio::test]
async fn configured_search_returns_honest_keyword_mode() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route(
                "/test/_search",
                axum::routing::post(|axum::Json(query): axum::Json<serde_json::Value>| async move {
                    if query["query"].get("knn").is_some() {
                        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, axum::Json(serde_json::json!({"error":"fixture has no vector index"})));
                    }
                    (axum::http::StatusCode::OK, axum::Json(serde_json::json!({"timed_out":false,"_shards":{"failed":0},"hits":{"total":{"value":0,"relation":"eq"},"hits":[]}})))
                }),
            ),
        )
        .await
        .unwrap();
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let app = service_guide::api::router_with_search(
        pool,
        Some(format!("http://{address}")),
        "test".into(),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/services?q=recycling")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["search_mode"], "keyword");
    server.abort();
}
