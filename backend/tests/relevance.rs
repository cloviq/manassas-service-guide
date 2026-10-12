use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

// A bounded labeled evaluation, not an assertion that all paraphrases work.
// Misses are reported in the output rather than failing the test.
#[tokio::test]
#[ignore = "requires real PostgreSQL, OpenSearch, and embedding service"]
async fn report_real_relevance_including_known_failure() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let url = std::env::var("OPENSEARCH_URL").unwrap();
    let index = std::env::var("OPENSEARCH_INDEX").unwrap_or_else(|_| "manassas-catalog".into());
    let app = service_guide::api::router_with_search(pool, Some(url.clone()), index.clone());
    let cases = [
        ("my garbage bin is broken", "Request a replacement cart"),
        ("replace damaged container", "Request a replacement cart"),
        ("when does the truck come", "Check collection schedules"),
        ("dispose of an old sofa", "Arrange a bulk item pickup"),
        (
            "my trash was not picked up",
            "Report missed trash collection",
        ),
        (
            "recycling pickup was missed",
            "Report missed recycling collection",
        ),
        (
            "yard waste was not collected",
            "Report missed yard waste collection",
        ),
        (
            "what day is garbage collection",
            "Check collection schedules",
        ),
    ];
    let mut rows = vec![];
    for (query, expected) in cases {
        let mut encoded_url = reqwest::Url::parse("http://localhost/api/services").unwrap();
        encoded_url
            .query_pairs_mut()
            .append_pair("q", query)
            .append_pair("type", "service")
            .append_pair("limit", "100");
        let encoded_uri = format!("{}?{}", encoded_url.path(), encoded_url.query().unwrap());
        let start = std::time::Instant::now();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(encoded_uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["search_mode"], "hybrid");
        assert!(v.get("warning").is_none());
        let items = v["items"].as_array().unwrap();
        let rank = items
            .iter()
            .position(|d| d["name"] == expected)
            .map(|i| i + 1);
        rows.push(json!({"query":query,"expected_top":expected,"actual_top":items.first().map(|d| &d["name"]),"expected_rank":rank,"top1_correct":rank==Some(1),"elapsed_ms":elapsed_ms,"total":v["total"]}));
    }
    let hits = rows.iter().filter(|r| r["top1_correct"] == true).count();
    let alias: Value = reqwest::get(format!("{url}/_alias/{index}"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let report = json!({"corpus_alias":alias,"model":std::env::var("EMBEDDING_MODEL").unwrap(),"queries":rows,"top1_hits":hits,"query_count":cases.len(),"top1_accuracy":hits as f64/cases.len() as f64,"measurement":"single sequential request per query, in-process Axum router plus real model/OpenSearch; not HTTP network or load-test latency"});
    let text = serde_json::to_string_pretty(&report).unwrap();
    println!("{text}");
    if let Ok(path) = std::env::var("RELEVANCE_REPORT") {
        std::fs::write(path, text).unwrap();
    }
}
