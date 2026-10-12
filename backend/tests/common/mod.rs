//! Expectations for the ignored live tests, generated from the downloaded catalog by
//! `scripts/catalog_expectations.py` (run by `make data-fetch`). Nothing here is
//! hardcoded to a particular snapshot of the public sources.
#![allow(dead_code)]
use serde_json::Value;
use std::path::Path;

fn read(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../catalog")
        .join(name);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "{}: {e}; run make data-fetch before the live tests",
            path.display()
        )
    }))
    .unwrap()
}
pub fn expectations() -> Value {
    let e = read("expectations.json");
    // Stale expectations would fail every assertion for the wrong reason.
    for (name, count) in e["records"].as_object().unwrap() {
        assert_eq!(
            read(name).as_array().unwrap().len() as u64,
            count.as_u64().unwrap(),
            "catalog/expectations.json is out of date for {name}; run python3 scripts/catalog_expectations.py"
        );
    }
    e
}
/// Every downloaded record the database should contain (base snapshot + map expansion).
pub fn catalog_records() -> Vec<Value> {
    let e = expectations();
    let mut records = Vec::new();
    for name in e["records"].as_object().unwrap().keys() {
        records.extend(read(name).as_array().unwrap().iter().cloned());
    }
    records
}
pub fn source_count(e: &Value, key: &str) -> usize {
    e["by_source"][key].as_u64().unwrap_or(0) as usize
}
pub fn layer_count(e: &Value, layer: &str) -> usize {
    e["map"]["by_layer"][layer].as_u64().unwrap_or(0) as usize
}
/// Viewport containing every mapped feature, padded slightly.
pub fn extent_bbox(e: &Value) -> String {
    let x: Vec<f64> = e["map"]["extent"]
        .as_array()
        .expect("catalog has mapped geometry")
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    format!(
        "{},{},{},{}",
        x[0] - 0.001,
        x[1] - 0.001,
        x[2] + 0.001,
        x[3] + 0.001
    )
}
pub fn in_extent(e: &Value, x: f64, y: f64) -> bool {
    let b: Vec<f64> = e["map"]["extent"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    (b[0]..=b[2]).contains(&x) && (b[1]..=b[3]).contains(&y)
}
/// A multi-segment street whose name is unambiguous in search.
pub struct StreetGroup {
    pub source_key: String,
    pub name: String,
    pub segments: usize,
}
pub fn street_group(e: &Value) -> StreetGroup {
    let g = &e["street_group"];
    assert!(
        g.is_object(),
        "catalog has no unambiguous multi-segment street"
    );
    StreetGroup {
        source_key: g["source_key"].as_str().unwrap().into(),
        name: g["name"].as_str().unwrap().into(),
        segments: g["segments"].as_u64().unwrap() as usize,
    }
}
pub fn encode(text: &str) -> String {
    text.replace('%', "%25")
        .replace(' ', "%20")
        .replace('&', "%26")
}
