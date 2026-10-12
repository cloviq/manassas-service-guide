use std::path::Path;

#[test]
fn migrations_define_sourced_catalog_and_private_ticket_storage() {
    let schema = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations/0001_catalog.sql"),
    )
    .unwrap_or_default();
    assert!(
        schema.contains("CREATE TABLE providers"),
        "provider persistence missing"
    );
    assert!(schema.contains("CREATE TABLE services"));
    assert!(schema.contains("CREATE TABLE tickets"));
    assert!(schema.contains("REFERENCES services(id)"));
    assert!(schema.contains("https://www.manassasva.gov/public_works/trash___recycling/index.php"));
    assert!(schema.contains("ON CONFLICT"));
}
