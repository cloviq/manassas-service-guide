use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn map_rejects_invalid_queries_before_database_access() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    let app = service_guide::app(pool);
    for uri in [
        "/api/map/features",
        "/api/map/features?bbox=NaN,38,-77,39",
        "/api/map/features?bbox=-78,39,-77,38",
        "/api/map/features?bbox=-181,38,-77,39",
        "/api/map/features?bbox=-78,38,-77,39&layers=water",
        "/api/map/features?bbox=-78,38,-77,39&layers=",
        "/api/map/features?bbox=-78,38,-77,39&limit=0",
        "/api/map/features?bbox=-78,38,-77,39&limit=2001",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=4",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=0,2,",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=-1",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=01",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=First",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=9&snow_types=0",
        "/api/map/features?bbox=-78,38,-77,39&snow_types=0&snow_types=1",
        "/api/map/features/not-a-uuid",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "{uri}");
    }
}
