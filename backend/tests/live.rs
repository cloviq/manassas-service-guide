// Explicit opt-in: requires real PostgreSQL and OpenSearch, never silently skipped.
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use std::process::Command;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires running local PostgreSQL and OpenSearch; run scripts/verify.sh"]
async fn real_cli_index_search_and_private_ticket_persistence() {
    let database = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let search = std::env::var("OPENSEARCH_URL").expect("OPENSEARCH_URL required");
    let index = std::env::var("OPENSEARCH_INDEX").unwrap_or("manassas-catalog".into());
    let exe = env!("CARGO_BIN_EXE_service-guide");
    for command in ["migrate", "index", "index"] {
        let result = Command::new(exe).arg(command).output().unwrap();
        assert!(
            result.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let pool = sqlx::PgPool::connect(&database).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM services")
        .fetch_one(&pool)
        .await
        .unwrap();
    let seeds: i64 = sqlx::query_scalar("SELECT count(*) FROM services WHERE source_key='seed'")
        .fetch_one(&pool)
        .await
        .unwrap();
    // Seed tasks come from migrations; any imported catalog records come on top.
    assert!(seeds > 0 && count >= seeds);
    let indexed: serde_json::Value = reqwest::get(format!("{search}/{index}/_count"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(indexed["count"], count);
    let app = service_guide::api::router_with_search(pool.clone(), Some(search), index);
    for (uri, mode, nonempty) in [
        ("/api/services", "database", true),
        ("/api/services?q=missed", "keyword", true),
        (
            "/api/services?q=cart&category=Trash%20%26%20recycling",
            "keyword",
            true,
        ),
        (
            "/api/services?q=cart&category=Water%20%26%20sewer",
            "keyword",
            false,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let value: serde_json::Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(value["search_mode"], mode);
        assert_eq!(!value["items"].as_array().unwrap().is_empty(), nonempty);
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/services/20000000-0000-4000-8000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let response = app.clone().oneshot(Request::builder().method("POST").uri("/api/tickets").header("content-type", "application/json").body(Body::from(r#"{"description":"Synthetic integration test, not a city request","service_id":"20000000-0000-4000-8000-000000000001"}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), 201);
    let ticket: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(ticket["status"], "new");
    assert_eq!(ticket["demo"], true);
    let id = uuid::Uuid::parse_str(ticket["id"].as_str().unwrap()).unwrap();
    let stored: (String, bool) = sqlx::query_as("SELECT status,demo FROM tickets WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, ("new".into(), true));
    println!(
        "Verified {count} source-backed services, idempotent CLI indexing, keyword/category search, service detail, persisted demo ticket {id}"
    );
    // Retain synthetic ticket for independent readback; never print user descriptions/emails.
}
