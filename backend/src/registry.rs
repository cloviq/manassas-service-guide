//! Source registry compiled in from catalog/sources.json, shared with the
//! catalog scripts and frontend. Edit that file, never source keys in code.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub publisher: Publisher,
    pub url_detail_fields: Vec<String>,
    pub map_layers: Vec<MapLayer>,
    pub sources: Vec<Source>,
}
#[derive(Debug, Deserialize)]
pub struct Publisher {
    pub arcgis_services_url: String,
    pub hub_url: String,
}
#[derive(Debug, Deserialize)]
pub struct MapLayer {
    pub id: String,
    pub geometry: String,
    pub variant_param: Option<String>,
    /// Layers returned when a map request omits `layers`.
    #[serde(default)]
    pub api_default: bool,
}
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Details {
    pub strings: Vec<String>,
    pub numbers: Vec<String>,
    pub nonnegative: Vec<String>,
    pub multiline: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct ResidentDescription {
    pub title_fields: Vec<String>,
    #[serde(default)]
    pub fallback_to_name: bool,
    pub template: String,
}
#[derive(Debug, Deserialize)]
pub struct SourceMap {
    pub layer: String,
    pub classification: String,
    pub variant: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct Source {
    pub key: String,
    #[serde(default)]
    pub details: Details,
    pub resident_description: Option<ResidentDescription>,
    #[serde(default)]
    pub source_link_fields: Vec<String>,
    #[serde(default)]
    pub caveats: BTreeMap<String, String>,
    #[serde(default)]
    pub street_grouping: bool,
    pub map: Option<SourceMap>,
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../catalog/sources.json"))
        .expect("catalog/sources.json must match the registry schema")
});
pub fn registry() -> &'static Registry {
    &REGISTRY
}
pub fn source(key: &str) -> Option<&'static Source> {
    REGISTRY.sources.iter().find(|s| s.key == key)
}
pub fn layer(id: &str) -> Option<&'static MapLayer> {
    REGISTRY.map_layers.iter().find(|l| l.id == id)
}
/// ArcGIS layer query endpoint for a `Family/layer` source key.
pub fn query_url(key: &str) -> Option<String> {
    let (family, layer) = key.split_once('/')?;
    Some(format!(
        "{}{family}/FeatureServer/{layer}/query",
        REGISTRY.publisher.arcgis_services_url
    ))
}
pub fn is_hub_url(url: &str) -> bool {
    url.trim_end_matches('/') == REGISTRY.publisher.hub_url.trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_parses_and_map_sources_reference_known_layers() {
        let r = registry();
        assert!(!r.sources.is_empty());
        for s in &r.sources {
            if let Some(map) = &s.map {
                let layer = layer(&map.layer).expect("map layer declared");
                assert_eq!(
                    layer.variant_param.is_some(),
                    map.variant.is_some(),
                    "{}",
                    s.key
                );
            }
            if !s.source_link_fields.is_empty() {
                assert!(query_url(&s.key).is_some());
            }
        }
    }
}
