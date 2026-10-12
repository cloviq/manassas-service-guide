use axum::{
    Json, Router,
    routing::{post, put},
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
#[tokio::test]
async fn indexing_uses_stable_document_ids_and_reads_back_count() {
    let bodies = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = bodies.clone();
    let app = Router::new()
        .route(
            "/test",
            put(|| async { Json(json!({"acknowledged":true})) }),
        )
        .route(
            "/test/_bulk",
            post(move |body: String| {
                let seen = seen.clone();
                async move {
                    seen.lock().unwrap().push(body);
                    Json(json!({"errors":false}))
                }
            }),
        )
        .route(
            "/test/_count",
            axum::routing::get(|| async { Json(json!({"count":1})) }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let doc:service_guide::api::Service=serde_json::from_value(json!({"id":"20000000-0000-4000-8000-000000000001","name":"test","description":"test description","category":"Trash & recycling","provider_name":"City","jurisdiction":"City of Manassas, Virginia","official_url":"https://www.manassasva.gov/","source_url":"https://www.manassasva.gov/","instructions":"Follow official source"})).unwrap();
    let documents = vec![doc];
    for _ in 0..2 {
        assert_eq!(
            service_guide::search::index_documents(
                &format!("http://{address}"),
                "test",
                &documents
            )
            .await
            .unwrap(),
            1
        );
    }
    let captured = bodies.lock().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0], captured[1]);
    let first: Value = serde_json::from_str(captured[0].lines().next().unwrap()).unwrap();
    assert_eq!(
        first["index"]["_id"],
        "20000000-0000-4000-8000-000000000001"
    );
    handle.abort();
}
