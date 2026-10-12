use crate::api::ApiError;
#[path = "esri.rs"]
mod esri;
use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[cfg(test)]
mod expansion_tests {
    use super::*;
    fn record(source: &str, geometry: Value) -> crate::api::Service {
        serde_json::from_value(json!({"id":uuid::Uuid::nil(),"name":"Published area","description":"Snapshot","category":"Environment","provider_name":"Publisher","jurisdiction":"City","official_url":"https://example.org","source_url":"https://example.org/1","instructions":"Verify","record_type":"place","source_key":source,"source_record_id":"1","attributes":{},"geometry":geometry})).unwrap()
    }
    #[test]
    fn polygon_crossing_rings_fail_closed_instead_of_inventing_topology() {
        let raw =
            json!({"rings":[[[0,0],[4,0],[4,4],[0,4],[0,0]],[[2,-1],[3,-1],[3,5],[2,5],[2,-1]]]});
        assert!(convert(record("Leaf_Collection_Sections/0", raw)).is_none());
    }
    #[test]
    fn point_polygon_closure_and_invalid_coordinates() {
        let point = convert(record(
            "Bike_Facilities_and_Racks/0",
            json!({"x":-77.5,"y":38.7,"spatialReference":{"wkid":4326}}),
        ))
        .unwrap();
        assert_eq!(point.feature["geometry"]["type"], "Point");
        assert_eq!(point.bounds, [-77.5, 38.7, -77.5, 38.7]);
        let polygon = convert(record(
            "Manassas_Bulk_Waste_Collection/0",
            json!({"rings":[[[0,0],[0,1],[1,1],[1,0]]]}),
        ))
        .unwrap();
        assert_eq!(polygon.feature["geometry"]["type"], "Polygon");
        assert_eq!(
            polygon.feature["geometry"]["coordinates"][0]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        for bad in [
            json!({"rings":[]}),
            json!({"rings":[[[0,0],[1,1],[2,2],[0,0]]]}),
            json!({"rings":[[[181,0],[1,1],[2,2],[181,0]]]}),
            json!({"rings":[[[0,0],[0,1],[1,1],[0,0]]],"spatialReference":{"wkid":3857}}),
        ] {
            assert!(convert(record("Leaf_Collection_Sections/0", bad)).is_none());
        }
        assert!(
            convert(record(
                "Bike_Facilities_and_Racks/0",
                json!({"x":-77.,"y":91.})
            ))
            .is_none()
        );
        assert!(
            convert(record(
                "Bike_Facilities_and_Racks/0",
                json!({"x":-77.,"y":38.,"spatialReference":{"wkid":3857,"latestWkid":4326}})
            ))
            .is_none()
        );
        assert!(convert(record("Unknown/0", json!({"x":-77.,"y":38.}))).is_none());
    }
    #[test]
    fn polygons_preserve_holes_disjoint_shells_and_nested_islands() {
        // Deliberately unordered and same winding: containment, not ordering, assigns holes.
        let raw = json!({"rings":[
            [[2,2],[8,2],[8,8],[2,8],[2,2]],
            [[20,0],[21,0],[21,1],[20,1],[20,0]],
            [[0,0],[10,0],[10,10],[0,10],[0,0]],
            [[3,3],[4,3],[4,4],[3,4],[3,3]]
        ]});
        let f = convert(record("Leaf_Collection_Sections/0", raw)).expect("polygon supported");
        assert_eq!(f.bounds, [0., 0., 21., 10.]);
        let g = &f.feature["geometry"];
        assert_eq!(g["type"], "MultiPolygon");
        let polys = g["coordinates"].as_array().unwrap();
        assert_eq!(polys.len(), 3);
        assert_eq!(
            polys
                .iter()
                .map(|p| p.as_array().unwrap().len())
                .sum::<usize>(),
            4
        );
        for polygon in polys {
            for (i, ring) in polygon.as_array().unwrap().iter().enumerate() {
                let pts = ring.as_array().unwrap();
                assert_eq!(pts.first(), pts.last());
                let area: f64 = pts
                    .windows(2)
                    .map(|p| {
                        p[0][0].as_f64().unwrap() * p[1][1].as_f64().unwrap()
                            - p[1][0].as_f64().unwrap() * p[0][1].as_f64().unwrap()
                    })
                    .sum();
                assert_eq!(area > 0., i == 0, "RFC7946 winding");
            }
        }
    }
}

#[derive(Deserialize)]
pub struct MapQuery {
    pub bbox: Option<String>,
    pub layers: Option<String>,
    pub limit: Option<usize>,
}
/// Raw query parameters, for registry-declared variant filters such as `snow_types`.
/// Kept as pairs so a repeated variant parameter is rejected rather than overwritten.
pub type RawQuery = Vec<(String, String)>;
/// Variant IDs of one map layer, in registry order.
fn layer_variants(layer: &str) -> Vec<&'static str> {
    crate::registry::registry()
        .sources
        .iter()
        .filter_map(|s| s.map.as_ref())
        .filter(|m| m.layer == layer)
        .filter_map(|m| m.variant.as_deref())
        .collect()
}
impl MapQuery {
    /// Selected (layer, variant) pairs for every layer that declares a variant parameter.
    fn variant_selection(raw: &RawQuery) -> Result<Vec<(&'static str, String)>, ApiError> {
        let mut selected = Vec::new();
        for layer in &crate::registry::registry().map_layers {
            let Some(param) = layer.variant_param.as_deref() else {
                continue;
            };
            let known = layer_variants(&layer.id);
            let mut values = raw.iter().filter(|(k, _)| k == param).map(|(_, v)| v);
            let requested = match (values.next(), values.next()) {
                (None, _) => known.join(","),
                (Some(value), None) => value.clone(),
                (Some(_), Some(_)) => {
                    return Err(ApiError(
                        StatusCode::BAD_REQUEST,
                        "Map variant filters must be given at most once.",
                    ));
                }
            };
            for variant in requested.split(',') {
                if !known.contains(&variant) {
                    return Err(ApiError(
                        StatusCode::BAD_REQUEST,
                        "Map variant filters must be nonempty comma-separated subsets of the layer's published variants.",
                    ));
                }
                selected.push((layer.id.as_str(), variant.to_owned()));
            }
        }
        Ok(selected)
    }
    fn validate(&self) -> Result<([f64; 4], Vec<&str>, usize), ApiError> {
        let bad = || {
            ApiError(
                StatusCode::BAD_REQUEST,
                "Expected bbox=west,south,east,north, layers from the published map layers and limit=1..2000.",
            )
        };
        let bounds: Vec<f64> = self
            .bbox
            .as_deref()
            .ok_or_else(bad)?
            .split(',')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .map_err(|_| bad())?;
        let b: [f64; 4] = bounds.try_into().map_err(|_| bad())?;
        if b.iter().any(|v| !v.is_finite())
            || b[0] < -180.
            || b[2] > 180.
            || b[1] < -90.
            || b[3] > 90.
            || b[0] >= b[2]
            || b[1] >= b[3]
        {
            return Err(bad());
        }
        let layers: Vec<&str> = match self.layers.as_deref() {
            Some(layers) => layers.split(',').collect(),
            None => crate::registry::registry()
                .map_layers
                .iter()
                .filter(|l| l.api_default)
                .map(|l| l.id.as_str())
                .collect(),
        };
        let limit = self.limit.unwrap_or(1000);
        if layers.iter().any(|l| crate::registry::layer(l).is_none())
            || !(1..=2000).contains(&limit)
        {
            return Err(bad());
        }
        Ok((b, layers, limit))
    }
}
// Per-router immutable read projection. No global cache shared between databases.
// Refresh by restarting the backend after a catalog import.
pub type MapCache = std::sync::Arc<tokio::sync::OnceCell<Vec<MapFeature>>>;
pub struct MapFeature {
    id: uuid::Uuid,
    group: Option<String>,
    layer: &'static str,
    variant: Option<&'static str>,
    bounds: [f64; 4],
    feature: Value,
}
fn convert(record: crate::api::Service) -> Option<MapFeature> {
    let map = crate::registry::source(&record.source_key)?.map.as_ref()?;
    let layer = crate::registry::layer(&map.layer)?;
    let (layer, classification, kind) = (
        layer.id.as_str(),
        map.classification.as_str(),
        layer.geometry.as_str(),
    );
    let raw = record.geometry.as_ref()?;
    let (geometry, bounds) = esri::geometry(raw, kind)?;
    let group = crate::projection::street_key(&record);
    Some(MapFeature {
        id: record.id,
        group,
        layer,
        variant: map.variant.as_deref(),
        bounds,
        feature: json!({
            "type":"Feature", "id":record.id, "geometry":geometry,
            "properties": {"id":record.id,"name":record.name,"layer":layer,"source_key":record.source_key,
            "source_record_id":record.source_record_id,"source_url":record.source_url,"classification":classification,
            "details":crate::projection::details(&record),"links":crate::projection::links(&record)}
        }),
    })
}
async fn load<'a>(
    cache: &'a MapCache,
    state: &crate::api::AppState,
) -> Result<&'a Vec<MapFeature>, ApiError> {
    cache
        .get_or_try_init(|| async {
            let mut tx = state.pool.begin().await?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
                .execute(&mut *tx)
                .await?;
            sqlx::query("SET LOCAL statement_timeout='5s'")
                .execute(&mut *tx)
                .await?;
            let keys: Vec<&str> = crate::registry::registry()
                .sources
                .iter()
                .filter(|s| s.map.is_some())
                .map(|s| s.key.as_str())
                .collect();
            let records = sqlx::query_as::<_, crate::api::Service>(&format!(
                "{} WHERE s.source_key = ANY($1) ORDER BY s.id LIMIT 10001",
                crate::api::SELECT_SERVICES
            ))
            .bind(&keys)
            .fetch_all(&mut *tx)
            .await?;
            if records.len() > 10000 {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Map projection bound exceeded.",
                ));
            }
            tx.commit().await?;
            // Fail closed rather than silently claim completeness if a source changes geometry shape.
            records
                .into_iter()
                .map(|r| {
                    convert(r).ok_or(ApiError(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "Unsupported map geometry in catalog.",
                    ))
                })
                .collect()
        })
        .await
}
fn collection(features: Vec<Value>, truncated: bool) -> Json<Value> {
    Json(json!({"type":"FeatureCollection", "features":features, "truncated":truncated}))
}
pub async fn features(
    axum::extract::State(state): axum::extract::State<crate::api::AppState>,
    axum::Extension(cache): axum::Extension<MapCache>,
    Query(query): Query<MapQuery>,
    Query(raw): Query<RawQuery>,
) -> Result<Json<Value>, ApiError> {
    let (b, layers, limit) = query.validate()?;
    let variants = MapQuery::variant_selection(&raw)?;
    let records = load(&cache, &state).await?;
    let mut matches = records.iter().filter(|f| {
        layers.contains(&f.layer)
            && f.variant
                .is_none_or(|v| variants.iter().any(|(l, s)| *l == f.layer && s == v))
            && f.bounds[0] <= b[2]
            && f.bounds[2] >= b[0]
            && f.bounds[1] <= b[3]
            && f.bounds[3] >= b[1]
    });
    let features = matches
        .by_ref()
        .take(limit)
        .map(|f| f.feature.clone())
        .collect();
    Ok(collection(features, matches.next().is_some()))
}
pub async fn detail(
    axum::extract::State(state): axum::extract::State<crate::api::AppState>,
    axum::Extension(cache): axum::Extension<MapCache>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let id = uuid::Uuid::parse_str(&id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "Invalid map feature ID."))?;
    let records = load(&cache, &state).await?;
    let record = records
        .iter()
        .find(|f| f.id == id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Map feature not found."))?;
    let features = records
        .iter()
        .filter(|f| f.id == id || (record.group.is_some() && f.group == record.group))
        .map(|f| f.feature.clone())
        .collect();
    Ok(collection(features, false))
}
