/// Read-only: every category button in the UI must have at least one
/// source-linked service task behind it, each with HTTPS official/source links.
#[tokio::test]
#[ignore = "read-only: requires migrated PostgreSQL"]
async fn every_ui_category_has_source_linked_services() {
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    for category in [
        "Trash & recycling",
        "Water & sewer",
        "Electricity",
        "Streets & sidewalks",
        "Other services",
    ] {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT name,official_url,source_url,instructions FROM services WHERE record_type='service' AND category=$1",
        )
        .bind(category)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert!(!rows.is_empty(), "no services in {category}");
        for (name, official, source, instructions) in rows {
            assert!(official.starts_with("https://"), "{name}");
            assert!(source.starts_with("https://"), "{name}");
            assert!(!instructions.trim().is_empty(), "{name}");
        }
    }
}
