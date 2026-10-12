//! Read-only imported catalog checks; no migrations, writes or reindex.
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
#[tokio::test]
#[ignore = "read-only: requires imported PostgreSQL catalog"]
async fn search_hydrates_details_from_postgres_without_reindex() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let record = sqlx::query_as::<_, service_guide::api::Service>(&format!(
        "{} WHERE s.source_key='Snow_Routes/0' ORDER BY s.id LIMIT 1",
        service_guide::api::SELECT_SERVICES
    ))
    .fetch_one(&pool)
    .await
    .unwrap();
    let expected = record.attributes["TYPE"].clone();
    let doc = serde_json::to_value(record).unwrap(); // index deliberately omits raw attributes
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, axum::Router::new().route("/test/_search", axum::routing::post(move || {
            let doc = doc.clone();
            async move { axum::Json(json!({"timed_out":false,"_shards":{"failed":0},"hits":{"total":{"value":1,"relation":"eq"},"hits":[{"_source":doc}]}})) }
        }))).await.unwrap();
    });
    let app = service_guide::api::router_with_search(
        pool,
        Some(format!("http://{address}")),
        "test".into(),
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/services?q=snow&type=route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let value: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["items"][0]["details"]["TYPE"], expected);
    server.abort();
}
#[tokio::test]
#[ignore = "read-only: requires imported PostgreSQL catalog"]
async fn every_stored_gis_link_and_detail_matches_source_schema() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let rows = sqlx::query_as::<_, service_guide::api::Service>(&format!(
        "{} WHERE s.source_key <> 'seed'",
        service_guide::api::SELECT_SERVICES
    ))
    .fetch_all(&pool)
    .await
    .unwrap();
    let mut checked = 0;
    for row in rows {
        let meta: Value = serde_json::from_slice(
            &std::fs::read(format!("../catalog/raw/{}/metadata.json", row.source_key))
                .or_else(|_| {
                    std::fs::read(format!(
                        "../catalog/map_expansion_raw/{}/metadata.json",
                        row.source_key
                    ))
                })
                .unwrap(),
        )
        .unwrap();
        let fields = meta["fields"].as_array().unwrap();
        let details = service_guide::projection::details(&row);
        for (key, value) in &details {
            if ["schedule_caveat", "agency_caveat"].contains(&key.as_str()) {
                continue;
            }
            assert!(
                fields.iter().any(|f| f["name"] == *key),
                "{} {key}",
                row.source_key
            );
            assert!(value.is_string() || value.is_number());
            let expected = row.attributes[key]
                .as_str()
                .map(|s| json!(s.trim()))
                .unwrap_or_else(|| row.attributes[key].clone());
            assert_eq!(value, &expected, "{} {key}", row.source_key);
        }
        let links = service_guide::projection::links(&row);
        let url = reqwest::Url::parse(&links.source_record).unwrap();
        assert_eq!(url.host_str(), Some("services1.arcgis.com"));
        let (family, layer) = row.source_key.split_once('/').unwrap();
        assert!(
            url.path()
                .ends_with(&format!("/{family}/FeatureServer/{layer}/query"))
        );
        assert!(
            url.query_pairs()
                .any(|(k, v)| k == "objectIds" && v == row.source_record_id)
        );
        assert!(links.service_action.is_none());
        checked += 1;
    }
    // Every imported catalog record (base snapshot + map expansion) was verified.
    let expected: usize = ["../catalog/records.json", "../catalog/map_expansion.json"]
        .iter()
        .map(|path| {
            serde_json::from_slice::<Vec<Value>>(&std::fs::read(path).unwrap())
                .unwrap()
                .len()
        })
        .sum();
    assert_eq!(checked, expected);
    println!(
        "Verified {checked} stored GIS records against source schema and exact layer/objectIds links"
    );
}
