#[test]
fn import_contract_rejects_duplicate_ids_and_preserves_unknown_operator() {
    let row = serde_json::json!({"id":"20000000-0000-4000-8000-000000000009","name":"Drain","description":"Mapped drain","category":"Drainage","provider_name":"","jurisdiction":"Manassas","official_url":"https://example.org","source_url":"https://example.org/data","instructions":"See source","record_type":"infrastructure","source_key":"gis","source_record_id":"9","attributes":{},"geometry":null});
    let docs =
        service_guide::catalog::parse_catalog(&serde_json::to_vec(&vec![row.clone()]).unwrap())
            .unwrap();
    assert_eq!(docs[0].provider_name, "Operator not verified");
    assert!(
        service_guide::catalog::parse_catalog(
            &serde_json::to_vec(&vec![row.clone(), row]).unwrap()
        )
        .is_err()
    );
}
