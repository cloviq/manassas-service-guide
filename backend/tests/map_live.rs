//! Read-only: uses imported PostgreSQL catalog, never migrates or reindexes.
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
mod common;
use service_guide::registry::{Source, registry};
async fn get(app: axum::Router, uri: &str) -> (u16, Value) {
    let r = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    (
        r.status().as_u16(),
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap(),
    )
}
/// Map sources on a layer, from the registry.
fn layer_sources(layer: &str) -> Vec<&'static Source> {
    registry()
        .sources
        .iter()
        .filter(|s| s.map.as_ref().is_some_and(|m| m.layer == layer))
        .collect()
}
#[tokio::test]
#[ignore = "read-only: requires imported PostgreSQL catalog"]
async fn viewport_is_spatial_bounded_allowlisted_and_detail_groups_members() {
    let e = common::expectations();
    let bbox = common::extent_bbox(&e);
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    let app = service_guide::app(pool.clone());
    // Omitting `layers` returns the registry's API default layers.
    let default_layers: Vec<&str> = registry()
        .map_layers
        .iter()
        .filter(|l| l.api_default)
        .map(|l| l.id.as_str())
        .collect();
    let total: usize = default_layers
        .iter()
        .map(|l| common::layer_count(&e, l))
        .sum();
    let (status, all) = get(
        app.clone(),
        &format!("/api/map/features?bbox={bbox}&limit=2000"),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(all["type"], "FeatureCollection");
    assert_eq!(all["features"].as_array().unwrap().len(), total.min(2000));
    assert_eq!(all["truncated"], total > 2000);
    let mut by_layer = std::collections::HashMap::new();
    for layer in &default_layers {
        let expected = common::layer_count(&e, layer);
        let (_, body) = get(
            app.clone(),
            &format!("/api/map/features?bbox={bbox}&layers={layer}&limit=2000"),
        )
        .await;
        assert_eq!(
            body["features"].as_array().unwrap().len(),
            expected.min(2000),
            "{layer}"
        );
        assert_eq!(body["truncated"], expected > 2000, "{layer}");
        by_layer.insert(*layer, body);
    }
    // Variant filters (snow route types) select exactly their sources, before the cap.
    for layer in registry()
        .map_layers
        .iter()
        .filter(|l| l.variant_param.is_some())
    {
        let param = layer.variant_param.as_deref().unwrap();
        let sources = layer_sources(&layer.id);
        let variant = |s: &Source| s.map.as_ref().unwrap().variant.clone().unwrap();
        let mut selections: Vec<Vec<&Source>> = sources.iter().map(|s| vec![*s]).collect();
        selections.push(sources.iter().step_by(2).copied().collect());
        for selection in selections {
            let ids: Vec<String> = selection.iter().map(|s| variant(s)).collect();
            let expected: usize = selection
                .iter()
                .map(|s| common::source_count(&e, &s.key))
                .sum();
            for query in [ids.join(","), format!("{0},{0}", ids.join(","))] {
                let (status, selected) = get(
                    app.clone(),
                    &format!(
                        "/api/map/features?bbox={bbox}&layers={}&{param}={query}&limit=2000",
                        layer.id
                    ),
                )
                .await;
                assert_eq!(status, 200);
                assert_eq!(
                    selected["features"].as_array().unwrap().len(),
                    expected,
                    "{param}={query}"
                );
                assert_eq!(selected["truncated"], false);
                for feature in selected["features"].as_array().unwrap() {
                    assert!(
                        selection
                            .iter()
                            .any(|s| feature["properties"]["source_key"] == s.key)
                    );
                }
            }
        }
        // The cap applies after filtering: exactly `count` fits, one fewer truncates.
        let source = sources
            .iter()
            .find(|s| common::source_count(&e, &s.key) > 1)
            .expect("a variant with several features");
        let count = common::source_count(&e, &source.key);
        let filter = format!(
            "bbox={bbox}&layers={}&{param}={}",
            layer.id,
            variant(source)
        );
        let (_, exact) = get(
            app.clone(),
            &format!("/api/map/features?{filter}&limit={count}"),
        )
        .await;
        assert_eq!(exact["truncated"], false);
        let (_, capped) = get(
            app.clone(),
            &format!("/api/map/features?{filter}&limit={}", count - 1),
        )
        .await;
        assert_eq!(capped["features"].as_array().unwrap().len(), count - 1);
        assert_eq!(capped["truncated"], true);
    }
    let keys: Vec<&str> = default_layers
        .iter()
        .flat_map(|l| layer_sources(l))
        .map(|s| s.key.as_str())
        .collect();
    let originals: Vec<(uuid::Uuid, Value)> =
        sqlx::query_as("SELECT id, geometry FROM services WHERE source_key = ANY($1)")
            .bind(&keys)
            .fetch_all(&pool)
            .await
            .unwrap();
    let originals: std::collections::HashMap<_, _> = originals.into_iter().collect();
    let mut counts = std::collections::BTreeMap::new();
    for feature in by_layer
        .values()
        .flat_map(|b| b["features"].as_array().unwrap())
    {
        let p = feature["properties"].as_object().unwrap();
        let id = uuid::Uuid::parse_str(p["id"].as_str().unwrap()).unwrap();
        let paths = &originals[&id]["paths"];
        let expected = if paths.as_array().unwrap().len() == 1 {
            &paths[0]
        } else {
            paths
        };
        assert_eq!(
            &feature["geometry"]["coordinates"], expected,
            "original XY/path preservation for {id}"
        );
        assert!(p["details"].is_object());
        assert_eq!(p.len(), 9);
        for key in p.keys() {
            assert!(
                [
                    "id",
                    "name",
                    "layer",
                    "source_key",
                    "source_record_id",
                    "source_url",
                    "classification",
                    "details",
                    "links"
                ]
                .contains(&key.as_str())
            );
        }
        assert!(
            ["LineString", "MultiLineString"]
                .contains(&feature["geometry"]["type"].as_str().unwrap())
        );
        assert!(p["source_url"].as_str().unwrap().contains("objectIds="));
        *counts
            .entry(p["source_key"].as_str().unwrap().to_owned())
            .or_insert(0) += 1;
    }
    // Exact per-source counts only where the layer response was not capped.
    for key in &keys {
        let layer = service_guide::registry::source(key)
            .unwrap()
            .map
            .as_ref()
            .unwrap()
            .layer
            .as_str();
        if by_layer[layer]["truncated"] == false {
            assert_eq!(
                counts.get(*key).copied().unwrap_or(0),
                common::source_count(&e, key),
                "{key}"
            );
        }
    }
    let (_, empty) = get(app.clone(), "/api/map/features?bbox=0,0,1,1").await;
    assert!(empty["features"].as_array().unwrap().is_empty());
    // A tiny viewport around one vertex of the sample street returns a handful of features.
    let street = common::street_group(&e);
    let ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        r"SELECT id FROM services WHERE source_key=$1 AND upper(regexp_replace(btrim(name), '\s+', ' ', 'g'))=$2 ORDER BY id",
    )
    .bind(&street.source_key)
    .bind(&street.name)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(ids.len(), street.segments);
    let vertex = &originals[&ids[0]]["paths"][0][0];
    let (x, y) = (vertex[0].as_f64().unwrap(), vertex[1].as_f64().unwrap());
    let (_, small) = get(
        app.clone(),
        &format!(
            "/api/map/features?bbox={},{},{},{}&limit=2000",
            x - 0.0005,
            y - 0.0005,
            x + 0.0005,
            y + 0.0005
        ),
    )
    .await;
    let n = small["features"].as_array().unwrap().len();
    assert!(n > 0 && n < 100, "small viewport count: {n}");
    for id in [&ids[0], ids.last().unwrap()] {
        let (status, detail) = get(app.clone(), &format!("/api/map/features/{id}")).await;
        assert_eq!(status, 200);
        let features = detail["features"].as_array().unwrap();
        assert_eq!(features.len(), ids.len());
        assert_eq!(detail["truncated"], false);
        for (f, id) in features.iter().zip(&ids) {
            assert_eq!(f["properties"]["id"], id.to_string());
        }
    }
    // A single (non-grouped) route resolves to exactly itself.
    for layer in registry()
        .map_layers
        .iter()
        .filter(|l| l.variant_param.is_some())
    {
        let feature = &by_layer[layer.id.as_str()]["features"][0];
        let (_, detail) = get(
            app.clone(),
            &format!("/api/map/features/{}", feature["id"].as_str().unwrap()),
        )
        .await;
        assert_eq!(detail["features"].as_array().unwrap().len(), 1);
    }
    // Seed services have no map geometry; unknown UUIDs are 404 too.
    let seed: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM services WHERE source_key='seed' LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        get(app.clone(), &format!("/api/map/features/{seed}"))
            .await
            .0,
        404
    );
    assert_eq!(
        get(
            app,
            "/api/map/features/00000000-0000-0000-0000-000000000000"
        )
        .await
        .0,
        404
    );
    println!(
        "{} default-layer features; variant filters before cap; small viewport {n}; all {} {} members; 9-field allowlist",
        total, street.segments, street.name
    );
}
