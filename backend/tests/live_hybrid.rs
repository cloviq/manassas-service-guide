use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
#[tokio::test]
#[ignore = "requires real PostgreSQL, OpenSearch, and embedding service"]
async fn real_model_cosine_bm25_and_filtering() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let url = std::env::var("OPENSEARCH_URL").unwrap();
    let index = std::env::var("OPENSEARCH_INDEX").unwrap_or_else(|_| "manassas-catalog".into());
    let app =
        service_guide::api::router_with_search(pool.clone(), Some(url.clone()), index.clone());
    let query = "replace damaged container";
    let lexical: Value = reqwest::Client::new()
        .post(format!("{url}/{index}/_search"))
        .json(&service_guide::search::lexical_query(query, "", "service"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        lexical["hits"]["total"]["value"], 0,
        "this query must have no BM25 AND match, so any hit comes from the semantic leg"
    );
    for (uri, mode) in [
        ("/api/services?q=replace%20damaged%20container", "hybrid"),
        (
            "/api/services?q=replace%20damaged%20container&limit=1&offset=1",
            "hybrid",
        ),
        ("/api/services?q=garbage&type=infrastructure", "hybrid"),
        (
            "/api/services?q=garbage&category=Water%20%26%20sewer",
            "hybrid",
        ),
        ("/api/services?limit=1&offset=1000", "database"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let v: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(v["search_mode"], mode);
        assert!(v.get("warning").is_none());
        if uri == "/api/services?q=replace%20damaged%20container" {
            assert_eq!(v["items"][0]["name"], "Request a replacement cart");
            // Relevance filtering must not return the unrelated seed services.
            assert_eq!(v["total"], 1);
        }
        if uri.contains("offset=1") && !uri.contains("offset=1000") {
            assert_eq!(v["total"], 1);
            assert!(v["items"].as_array().unwrap().is_empty());
        }
        if uri.contains("type=infrastructure") {
            // A type filter constrains candidates; it does not make every
            // infrastructure record relevant to garbage.
            assert_eq!(v["total"], 0);
            assert!(
                v["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|d| d["record_type"] == "infrastructure" && d.get("attributes").is_none())
            );
        }
        if uri.contains("category=Water") {
            // A category filter constrains candidates; it does not make the
            // Water & sewer services (stormwater, water problems) relevant to garbage.
            let services: i64 = sqlx::query_scalar("SELECT count(*) FROM services WHERE record_type='service' AND category='Water & sewer'").fetch_one(&pool).await.unwrap();
            assert!(services > 0);
            assert_eq!(v["total"], 0);
        }
        if uri.contains("offset=1000") {
            assert_eq!(v["items"].as_array().unwrap().len(), 0);
            assert!(v["total"].as_u64().unwrap() >= 6);
        }
        println!(
            "{uri}: mode={}, total={}, first={}",
            v["search_mode"], v["total"], v["items"][0]["name"]
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM services")
        .fetch_one(&pool)
        .await
        .unwrap();
    let indexed: Value = reqwest::get(format!("{url}/{index}/_count"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(indexed["count"], count);
}
