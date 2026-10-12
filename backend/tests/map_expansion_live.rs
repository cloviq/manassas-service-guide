//! Imported-catalog read-only acceptance checks; never starts a listener.
use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
mod common;
use service_guide::registry::{layer, registry};
async fn get(app: axum::Router, uri: &str) -> (u16, Value) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    (
        response.status().as_u16(),
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap(),
    )
}
async fn pool() -> sqlx::PgPool {
    sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap()
}
fn vertex_set(value: &Value, out: &mut std::collections::HashSet<(u64, u64)>) {
    let a = value.as_array().unwrap();
    if a.first().is_some_and(Value::is_number) {
        out.insert((
            a[0].as_f64().unwrap().to_bits(),
            a[1].as_f64().unwrap().to_bits(),
        ));
    } else {
        for child in a {
            vertex_set(child, out);
        }
    }
}
fn check_coordinates(e: &Value, value: &Value) {
    let a = value.as_array().unwrap();
    if a.first().is_some_and(Value::is_number) {
        assert_eq!(a.len(), 2);
        assert!(common::in_extent(
            e,
            a[0].as_f64().unwrap(),
            a[1].as_f64().unwrap()
        ));
    } else {
        for child in a {
            check_coordinates(e, child);
        }
    }
}
#[tokio::test]
#[ignore = "read-only: requires imported PostgreSQL catalog"]
async fn imported_map_layers_have_exact_ids_safe_details_and_real_geometry() {
    let e = common::expectations();
    let bbox = common::extent_bbox(&e);
    let pool = pool().await;
    let catalog = common::catalog_records();
    let present: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM services")
        .fetch_all(&pool)
        .await
        .unwrap();
    let present: std::collections::HashSet<String> =
        present.iter().map(|id| id.to_string()).collect();
    for r in &catalog {
        assert!(
            present.contains(r["id"].as_str().unwrap()),
            "missing imported record {}",
            r["id"]
        );
    }
    let app = service_guide::app(pool.clone());
    // Every single-source overlay layer; default street/snow layers are covered by map_live.
    for source in registry().sources.iter().filter(|s| {
        s.map.as_ref().is_some_and(|m| {
            let l = layer(&m.layer).unwrap();
            !l.api_default && l.variant_param.is_none()
        })
    }) {
        let map = source.map.as_ref().unwrap();
        let (layer, kind) = (
            map.layer.as_str(),
            layer(&map.layer).unwrap().geometry.as_str(),
        );
        let source = source.key.as_str();
        let count = common::source_count(&e, source);
        let spec = service_guide::registry::source(source).unwrap();
        let allow: Vec<&str> = spec
            .details
            .strings
            .iter()
            .chain(&spec.details.numbers)
            .chain(&spec.details.nonnegative)
            .chain(spec.caveats.keys())
            .map(String::as_str)
            .collect();
        let (status, body) = get(
            app.clone(),
            &format!("/api/map/features?bbox={bbox}&layers={layer}&limit=2000"),
        )
        .await;
        assert_eq!(status, 200, "{layer}: {body}");
        let features = body["features"].as_array().unwrap();
        assert_eq!(features.len(), count, "{layer}");
        assert_eq!(body["truncated"], false);
        let expected: std::collections::HashSet<String> = catalog
            .iter()
            .filter(|r| r["source_key"] == source)
            .map(|r| r["id"].as_str().unwrap().to_string())
            .collect();
        let actual: std::collections::HashSet<_> = features
            .iter()
            .map(|f| f["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(actual, expected);
        for f in features {
            let p = &f["properties"];
            assert_eq!(p.as_object().unwrap().len(), 9);
            assert_eq!(p["source_key"], source);
            for key in p["details"].as_object().unwrap().keys() {
                assert!(allow.contains(&key.as_str()), "{source}: {key}");
            }
            assert!(
                p["links"]["source_record"]
                    .as_str()
                    .unwrap()
                    .contains("objectIds=")
            );
            assert!(
                !p["links"]["source_record"]
                    .as_str()
                    .unwrap()
                    .contains("%2A")
            );
            let g = &f["geometry"];
            match kind {
                "polygon" => {
                    assert!(["Polygon", "MultiPolygon"].contains(&g["type"].as_str().unwrap()));
                }
                "point" => assert_eq!(g["type"], "Point"),
                _ => assert!(
                    ["LineString", "MultiLineString"].contains(&g["type"].as_str().unwrap())
                ),
            }
            check_coordinates(&e, &g["coordinates"]);
            let raw: Value = sqlx::query_scalar("SELECT geometry FROM services WHERE id=$1")
                .bind(uuid::Uuid::parse_str(f["id"].as_str().unwrap()).unwrap())
                .fetch_one(&pool)
                .await
                .unwrap();
            let raw_coordinates = if kind == "point" {
                serde_json::json!([raw["x"], raw["y"]])
            } else if kind == "polygon" {
                raw["rings"].clone()
            } else {
                raw["paths"].clone()
            };
            let (mut original, mut converted) = (
                std::collections::HashSet::new(),
                std::collections::HashSet::new(),
            );
            vertex_set(&raw_coordinates, &mut original);
            vertex_set(&g["coordinates"], &mut converted);
            assert_eq!(original, converted, "all original XY vertices preserved");
            if kind == "polygon" {
                let rings: Vec<&Value> = if g["type"] == "Polygon" {
                    g["coordinates"].as_array().unwrap().iter().collect()
                } else {
                    g["coordinates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .flat_map(|p| p.as_array().unwrap())
                        .collect()
                };
                assert_eq!(rings.len(), raw["rings"].as_array().unwrap().len());
                for ring in rings {
                    let r = ring.as_array().unwrap();
                    assert_eq!(r.first(), r.last());
                }
            }
            let &(x, y) = original.iter().next().unwrap();
            let (x, y) = (f64::from_bits(x), f64::from_bits(y));
            let (status, local) = get(
                app.clone(),
                &format!(
                    "/api/map/features?bbox={},{},{},{}&layers={layer}&limit=2000",
                    x - 0.000001,
                    y - 0.000001,
                    x + 0.000001,
                    y + 0.000001
                ),
            )
            .await;
            assert_eq!(status, 200);
            assert!(
                local["features"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["id"] == f["id"])
            );
            for (key, caveat) in &spec.caveats {
                assert_eq!(p["details"][key], *caveat);
            }
            let (status, detail) = get(
                app.clone(),
                &format!("/api/map/features/{}", f["id"].as_str().unwrap()),
            )
            .await;
            assert_eq!(status, 200);
            assert_eq!(detail["features"][0], *f);
            assert_eq!(detail["features"].as_array().unwrap().len(), 1);
        }
        let (status, empty) = get(
            app.clone(),
            &format!("/api/map/features?bbox=0,0,1,1&layers={layer}"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(empty["features"].as_array().unwrap().is_empty());
        let (_, limited) = get(
            app.clone(),
            &format!("/api/map/features?bbox={bbox}&layers={layer}&limit=1"),
        )
        .await;
        assert_eq!(limited["truncated"], count > 1);
        println!(
            "{layer}: {count} exact IDs; geometry, details allowlist, links, detail, empty viewport and cap"
        );
    }
    println!(
        "{} downloaded catalog records present in the database",
        catalog.len()
    );
}
