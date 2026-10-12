#[test]
fn validate_import_is_offline_and_has_no_database_side_effects() {
    let path = std::env::temp_dir().join(format!("catalog-fixture-{}.json", uuid::Uuid::new_v4()));
    let row = serde_json::json!({"id":"20000000-0000-4000-8000-000000000009","name":"Fixture only","description":"Not production data","category":"Test","jurisdiction":"Test","official_url":"https://example.org","source_url":"https://example.org/data","instructions":"See source","record_type":"place","source_key":"fixture","source_record_id":"9","attributes":{},"geometry":null});
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({"records":[row]})).unwrap(),
    )
    .unwrap();
    let run = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_service-guide"))
            .args(["validate-import", path.to_str().unwrap()])
            .env_remove("DATABASE_URL")
            .output()
            .unwrap()
    };
    let valid = run();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    assert!(String::from_utf8_lossy(&valid.stdout).contains("Validated 1"));
    std::fs::write(&path, b"{bad json").unwrap();
    assert!(!run().status.success());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn cli_has_explicit_serve_migrate_index_commands_and_requires_configuration() {
    let exe = env!("CARGO_BIN_EXE_service-guide");
    let help = std::process::Command::new(exe)
        .arg("--help")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&help.stdout).contains("serve | migrate | index"));
    let no_db = std::process::Command::new(exe)
        .arg("serve")
        .env_remove("DATABASE_URL")
        .output()
        .unwrap();
    assert!(!no_db.status.success());
    assert!(String::from_utf8_lossy(&no_db.stderr).contains("DATABASE_URL"));
    let no_search = std::process::Command::new(exe)
        .arg("index")
        .env(
            "DATABASE_URL",
            "postgres://unused:unused@127.0.0.1:1/unused",
        )
        .env_remove("OPENSEARCH_URL")
        .output()
        .unwrap();
    assert!(!no_search.status.success());
    assert!(String::from_utf8_lossy(&no_search.stderr).contains("OPENSEARCH_URL"));
}
