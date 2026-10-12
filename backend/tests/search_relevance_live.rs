use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "read-only real PostgreSQL/OpenSearch/embedding regression"]
async fn reject_unrelated_candidates_preserve_snow_routes_and_semantic_cart() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let app = service_guide::api::router_with_search(
        pool,
        Some(std::env::var("OPENSEARCH_URL").unwrap()),
        std::env::var("OPENSEARCH_INDEX").unwrap(),
    );
    for (q, kind, expected) in [
        // Snow is a real service since migration 0005; it must still never
        // surface the unrelated trash tasks.
        ("snow", "service", Some("service")),
        // Everyday word absent from City text; reaches trash via civic synonyms.
        ("garbage", "service", Some("service")),
        ("qzxvplmokn", "all", None),
        ("quantum banana spaceship", "all", None),
        ("pizza delivery", "all", None),
        ("buy concert tickets", "all", None),
        ("passport renewal", "all", None),
        // Actionable snow services rank first; all snow routes stay findable.
        ("snow", "all", Some("service")),
        ("snow", "route", Some("route")),
        ("replace damaged container", "service", Some("service")),
    ] {
        let mut url = reqwest::Url::parse("http://localhost/api/services").unwrap();
        url.query_pairs_mut()
            .append_pair("q", q)
            .append_pair("type", kind)
            .append_pair("limit", "1");
        let uri = format!("{}?{}", url.path(), url.query().unwrap());
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let v: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        println!(
            "{q}/{kind}: total={} first={}",
            v["total"], v["items"][0]["name"]
        );
        assert_eq!(v["search_mode"], "hybrid");
        assert_eq!(v["total_scope"], "lexical_plus_bounded_semantic_candidates");
        assert_eq!(v["semantic_candidate_limit"], 100);
        assert!(v.get("warning").is_none());
        if let Some(expected_type) = expected {
            assert!(v["total"].as_u64().unwrap() > 0, "{q}");
            assert_eq!(v["items"][0]["record_type"], expected_type);
            if q == "replace damaged container" {
                assert_eq!(v["items"][0]["name"], "Request a replacement cart");
            }
            if (q, kind) == ("snow", "service") {
                // Exactly the three snow tasks: no trash service leaks in.
                assert_eq!(v["total"], 3, "{q}");
                assert_eq!(v["items"][0]["category"], "Streets & sidewalks");
            }
            if (q, kind) == ("snow", "all") {
                assert!(
                    v["total"].as_u64().unwrap() > 300,
                    "snow routes still returned"
                );
            }
            if q == "garbage" {
                assert_eq!(v["items"][0]["category"], "Trash & recycling");
            }
        } else {
            assert_eq!(v["total"], 0, "unrelated query {q}");
            assert!(v["items"].as_array().unwrap().is_empty());
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("{uri}&offset=100000"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let page: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(page["total"], v["total"]);
        assert!(page["items"].as_array().unwrap().is_empty());
    }
}
