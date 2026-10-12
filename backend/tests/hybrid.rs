use serde_json::json;
use service_guide::search::*;
#[test]
fn hybrid_filters_both_legs_and_fuses_deterministically() {
    let a = lexical_query("bin", "Trash", "service");
    let b = vector_query(&[1.0, 0.0], "Trash", "service");
    assert_eq!(
        a["query"]["bool"]["filter"],
        b["query"]["knn"]["embedding"]["filter"]["bool"]["filter"]
    );
    assert_eq!(b["query"]["knn"]["embedding"]["k"], 100);
    assert_eq!(b["size"], 100);
    assert_eq!(b["min_score"], (1.0 + MIN_SEMANTIC_COSINE) / 2.0);
    assert!(b["query"].get("script_score").is_none());
    assert_eq!(
        rrf_ids(&["b".into(), "a".into()], &["a".into(), "c".into()]),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        index_mapping(384, "model")["mappings"]["properties"]["embedding"]["dimension"],
        json!(384)
    );
    assert_eq!(
        index_mapping(384, "model")["mappings"]["_meta"]["search_schema"],
        json!(SEARCH_SCHEMA_VERSION)
    );
    let analysis = &index_mapping(384, "model")["settings"]["analysis"];
    assert_eq!(
        analysis["analyzer"]["civic_search"]["filter"],
        json!(["lowercase", "civic_synonyms"])
    );
    assert!(
        analysis["filter"]["civic_synonyms"]["synonyms"]
            .as_array()
            .unwrap()
            .contains(&json!("garbage, trash, refuse, rubbish"))
    );
}

#[test]
fn expanded_record_roundtrips_provenance_without_inventing_operator() {
    let value = json!({"id":"20000000-0000-4000-8000-000000000009","name":"Drain","description":"Mapped drain","category":"Drainage","provider_name":"Operator not verified","jurisdiction":"Manassas","official_url":"https://example.org","source_url":"https://example.org/data","instructions":"See source","record_type":"infrastructure","source_key":"gis","source_record_id":"9","attributes":{"publisher":"City"},"geometry":{"type":"Point","coordinates":[-77.0,38.0]}});
    let doc: service_guide::api::Service = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(doc.attributes, value["attributes"]);
    let public = serde_json::to_value(doc).unwrap();
    assert!(
        public.get("attributes").is_none(),
        "internal raw attributes must not reach public clients"
    );
    assert_eq!(public["source_key"], value["source_key"]);
    assert_eq!(public["geometry"], value["geometry"]);
}

#[tokio::test]
async fn embeddings_reject_wrong_dimension_and_order_by_index() {
    let app=axum::Router::new().route("/v1/embeddings",axum::routing::post(||async { axum::Json(json!({"model":"test","data":[{"index":1,"embedding":[0.0,1.0]},{"index":0,"embedding":[1.0,0.0]}]})) }));
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let h = tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    let mut c = EmbeddingConfig {
        url: format!("http://{addr}/v1/embeddings"),
        model: "test".into(),
        dimension: 2,
        batch_size: 32,
    };
    let client = reqwest::Client::new();
    let input = vec!["a".into(), "b".into()];
    assert_eq!(
        c.embed(&client, &input).await.unwrap(),
        vec![vec![1.0, 0.0], vec![0.0, 1.0]]
    );
    c.dimension = 384;
    assert!(c.embed(&client, &input).await.is_err());
    h.abort();
}

#[tokio::test]
async fn invalid_type_and_pagination_rejected_before_database() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(5))
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    for uri in [
        "/api/services?type=bogus",
        "/api/services?limit=101",
        "/api/services?offset=100001",
    ] {
        let r = service_guide::app(pool.clone())
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), 400);
    }
}

#[tokio::test]
async fn mismatched_count_never_switches_alias() {
    let app = axum::Router::new().route(
        "/physical/_count",
        axum::routing::get(|| async { axum::Json(json!({"count":0})) }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let h = tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    assert!(
        publish_index(
            &reqwest::Client::new(),
            &format!("http://{addr}"),
            "alias",
            "physical",
            1
        )
        .await
        .is_err()
    );
    h.abort();
}

#[tokio::test]
async fn timed_out_failed_shards_and_nonexact_totals_are_rejected() {
    for response in [
        json!({"timed_out":true,"hits":{"total":{"value":0,"relation":"eq"},"hits":[]}}),
        json!({"_shards":{"failed":1},"hits":{"total":{"value":0,"relation":"eq"},"hits":[]}}),
        json!({"hits":{"total":{"value":0,"relation":"gte"},"hits":[]}}),
        json!({"hits":{"hits":[]}}),
    ] {
        let app = axum::Router::new().route(
            "/test/_search",
            axum::routing::post(move || {
                let v = response.clone();
                async move { axum::Json(v) }
            }),
        );
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        let h = tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
        assert!(
            search_catalog(
                &reqwest::Client::new(),
                &format!("http://{addr}"),
                "test",
                None,
                "bin",
                "",
                "service"
            )
            .await
            .is_err()
        );
        h.abort();
    }
}

#[tokio::test]
async fn incomplete_search_is_not_reported_as_accurate_total() {
    let app = axum::Router::new().route(
        "/test/_search",
        axum::routing::post(|| async {
            axum::Json(json!({"hits":{"total":{"value":2,"relation":"eq"},"hits":[]}}))
        }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let h = tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    assert!(
        search_catalog(
            &reqwest::Client::new(),
            &format!("http://{addr}"),
            "test",
            None,
            "bin",
            "",
            "service"
        )
        .await
        .is_err()
    );
    h.abort();
}
