//! Read-only integration checks against the imported catalog; never reindexes or migrates.
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
mod common;
async fn get(app: axum::Router, uri: &str) -> Value {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
#[tokio::test]
#[ignore = "read-only: requires imported PostgreSQL catalog"]
async fn database_groups_before_counts_and_pagination_and_preserves_details() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let street = common::street_group(&common::expectations());
    let q = common::encode(&street.name);
    let app = service_guide::api::router(pool.clone());
    let uri = format!("/api/services?type=route&q={q}&limit=1");
    let uri = uri.as_str();
    let v = get(app.clone(), uri).await;
    assert_eq!(v["total"], 1);
    assert_eq!(v["items"][0]["segment_count"], street.segments);
    let next = get(app.clone(), &format!("{uri}&offset=1")).await;
    assert_eq!(next["total"], 1);
    assert_eq!(next["items"].as_array().unwrap().len(), 0);
    let wrong_type = get(app.clone(), &format!("/api/services?type=place&q={q}")).await;
    assert_eq!(wrong_type["total"], 0);
    let wrong_category = get(app.clone(), &format!("{uri}&category=Electricity")).await;
    assert_eq!(wrong_category["total"], 0);
    let detail = get(
        app.clone(),
        &format!("/api/services/{}", v["items"][0]["id"].as_str().unwrap()),
    )
    .await;
    let segments = detail["segments"].as_array().unwrap();
    assert_eq!(segments.len(), street.segments);
    for s in segments {
        assert!(s["source_url"].as_str().unwrap().contains("objectIds="));
        assert!(s.get("attributes").is_none());
    }
    let raw: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM services WHERE source_key=$1 AND upper(regexp_replace(btrim(name), '\\s+', ' ', 'g'))=$2",
    )
    .bind(&street.source_key)
    .bind(&street.name)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(raw as usize, street.segments);
    // Independently compute all-resource count in SQL, not via the Rust projection.
    let expected: i64 = sqlx::query_scalar("SELECT count(DISTINCT CASE WHEN source_key='Centerline_COM/0' AND record_type='route' AND btrim(name)<>'' THEN 'street:' || upper(regexp_replace(btrim(name), '\\s+', ' ', 'g')) ELSE 'id:' || id::text END) FROM services").fetch_one(&pool).await.unwrap();
    let all = get(app.clone(), "/api/services?type=all&limit=20").await;
    assert_eq!(all["total"], expected);
    assert_eq!(all["items"].as_array().unwrap().len(), 20);
    // Grouped rows report every member segment, matching an independent SQL count.
    for item in all["items"].as_array().unwrap() {
        if let Some(n) = item["segment_count"].as_i64() {
            let members: i64 = sqlx::query_scalar("SELECT count(*) FROM services WHERE source_key=$1 AND record_type='route' AND upper(regexp_replace(btrim(name), '\\s+', ' ', 'g'))=upper(regexp_replace(btrim($2), '\\s+', ' ', 'g'))")
                .bind(item["source_key"].as_str().unwrap())
                .bind(item["name"].as_str().unwrap())
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(n, members, "{}", item["name"]);
        }
    }
    let next = get(app.clone(), "/api/services?type=all&limit=20&offset=20").await;
    assert_eq!(next["total"], expected);
    let ids: std::collections::HashSet<_> = all["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(
        next["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| !ids.contains(r["id"].as_str().unwrap()))
    );
    let another = get(
        app,
        &format!(
            "/api/services/{}",
            segments.last().unwrap()["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(another["segment_count"], street.segments);
    println!(
        "database: {expected} resident resources; {} grouped from {} segments with provenance; exact paginated count",
        street.name, street.segments
    );
}
#[tokio::test]
#[ignore = "read-only: requires PostgreSQL, OpenSearch and real embeddings"]
async fn hybrid_groups_before_counts_and_pagination() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let expected: i64 = sqlx::query_scalar("SELECT count(DISTINCT CASE WHEN source_key='Centerline_COM/0' AND btrim(name)<>'' THEN 'street:' || upper(regexp_replace(btrim(name), '\\s+', ' ', 'g')) ELSE 'id:' || id::text END) FROM services WHERE record_type='route' AND category='Streets & sidewalks'").fetch_one(&pool).await.unwrap();
    let street = common::street_group(&common::expectations());
    let app = service_guide::api::router_with_search(
        pool,
        Some(std::env::var("OPENSEARCH_URL").unwrap()),
        std::env::var("OPENSEARCH_INDEX").unwrap_or_else(|_| "manassas-catalog".into()),
    );
    let uri = format!(
        "/api/services?type=route&q={}&category=Streets%20%26%20sidewalks&limit=100",
        common::encode(&street.name)
    );
    let uri = uri.as_str();
    let first = get(app.clone(), uri).await;
    assert_eq!(first["search_mode"], "hybrid");
    // Search totals count relevant projected candidates, not every street.
    assert!(expected > 1);
    assert_eq!(first["total"], 1);
    let hits: Vec<_> = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["name"].as_str().unwrap().to_uppercase() == street.name)
        .collect();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["segment_count"], street.segments);
    let small = get(app.clone(), &uri.replace("limit=100", "limit=1")).await;
    assert_eq!(small["total"], first["total"]);
    let next = get(
        app,
        &format!("{}&offset=1", uri.replace("limit=100", "limit=1")),
    )
    .await;
    assert_eq!(next["total"], 1);
    assert!(next["items"].as_array().unwrap().is_empty());
    println!(
        "hybrid: total={}, one {}-segment {} group; stable pagination",
        first["total"], street.segments, street.name
    );
}
