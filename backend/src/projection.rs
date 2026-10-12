//! Resident read model only. Raw database rows and search documents remain untouched.
use crate::api::Service;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Segment {
    pub id: Uuid,
    pub source_record_id: String,
    pub source_url: String,
    pub geometry: Option<serde_json::Value>,
    pub details: Details,
}
#[derive(Debug, Serialize)]
pub struct Resource {
    #[serde(flatten)]
    pub record: Service,
    pub details: Details,
    pub links: Links,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_count: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<Segment>,
}
/// Safe source fields retain their published names and JSON scalar types.
pub type Details = std::collections::BTreeMap<String, serde_json::Value>;
#[derive(Debug, Serialize)]
pub struct Links {
    pub source_record: String,
    pub service_information: Option<String>,
    // No action endpoint has been verified for these GIS records.
    pub service_action: Option<String>,
}
pub fn links(record: &Service) -> Links {
    let information = (record.official_url != record.source_url
        && !crate::registry::is_hub_url(&record.official_url))
    .then(|| record.official_url.clone());
    let narrowed = crate::registry::source(&record.source_key)
        .filter(|source| !source.source_link_fields.is_empty())
        .and_then(|source| {
            let mut url = reqwest::Url::parse(&crate::registry::query_url(&source.key)?).ok()?;
            url.query_pairs_mut()
                .append_pair("f", "json")
                .append_pair("objectIds", &record.source_record_id)
                .append_pair("outFields", &source.source_link_fields.join(","))
                .append_pair("returnGeometry", "true")
                .append_pair("outSR", "4326");
            Some(url.into())
        });
    Links {
        source_record: narrowed.unwrap_or_else(|| record.source_url.clone()),
        service_information: information,
        service_action: None,
    }
}
pub fn details(record: &Service) -> Details {
    let mut out = Details::new();
    let Some(source) = crate::registry::source(&record.source_key) else {
        return out;
    };
    let fields = &source.details;
    for key in &fields.strings {
        if let Some(value) = record
            .attributes
            .get(key)
            .and_then(serde_json::Value::as_str)
        {
            let value = value.trim();
            if !value.is_empty()
                && value.len() <= 2000
                && !value.chars().any(|c| {
                    c.is_control() && !(fields.multiline.contains(key) && ['\n', '\r'].contains(&c))
                })
            {
                out.insert(key.clone(), serde_json::json!(value));
            }
        }
    }
    for key in &fields.numbers {
        if let Some(value) = record.attributes.get(key).filter(|v| {
            v.as_f64()
                .is_some_and(|n| n.is_finite() && n > 0.0 && n.fract() == 0.0)
        }) {
            out.insert(key.clone(), value.clone());
        }
    }
    for key in &crate::registry::registry().url_detail_fields {
        if out
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| {
                reqwest::Url::parse(value).map_or(true, |url| {
                    url.scheme() != "https"
                        || url.host_str().is_none()
                        || !url.username().is_empty()
                        || url.password().is_some()
                })
            })
        {
            out.remove(key);
        }
    }
    for key in &fields.nonnegative {
        if let Some(value) = record
            .attributes
            .get(key)
            .filter(|v| v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.))
        {
            out.insert(key.clone(), value.clone());
        }
    }
    for (key, caveat) in &source.caveats {
        out.insert(key.clone(), serde_json::json!(caveat));
    }
    out
}
/// Resident-facing description from the registry template, if the source has one.
fn resident_description(record: &Service, details: &Details) -> Option<String> {
    let source = crate::registry::source(&record.source_key)?;
    let template = source.resident_description.as_ref()?;
    let title = template
        .title_fields
        .iter()
        .filter_map(|key| details.get(key).and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    let title = match (title.is_empty(), template.fallback_to_name) {
        (false, _) => title,
        (true, true) => record.name.clone(),
        (true, false) => return None,
    };
    let classification = source
        .map
        .as_ref()
        .map_or("", |m| m.classification.as_str());
    Some(
        template
            .template
            .replace("{title}", &title)
            .replace("{classification}", classification),
    )
}
/// Group only registry layers marked for street grouping, never arbitrary same-title resources.
/// Normalize case and whitespace but do not strip street suffixes or punctuation.
pub fn street_key(record: &Service) -> Option<String> {
    let name = record
        .name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase();
    (crate::registry::source(&record.source_key).is_some_and(|s| s.street_grouping)
        && record.record_type == "route"
        && !name.is_empty())
    .then_some(name)
}
/// Input order is database order or fused search rank. Keep the first rank of a
/// street, but select its smallest UUID as a stable detail link across modes.
/// Must run over ALL filtered candidates, before total/offset/limit.
pub fn project(records: Vec<Service>) -> Vec<Resource> {
    let mut out: Vec<Resource> = Vec::new();
    let mut groups: HashMap<String, usize> = HashMap::new();
    let mut seen = HashSet::new();
    for mut record in records {
        if !seen.insert(record.id) {
            continue;
        }
        let details = details(&record);
        let links = links(&record);
        if let Some(description) = resident_description(&record, &details) {
            record.description = description;
        }
        if crate::registry::is_hub_url(&record.official_url) {
            record.official_url = links.source_record.clone();
        }
        if let Some(key) = street_key(&record) {
            let segment = Segment {
                id: record.id,
                source_record_id: record.source_record_id.clone(),
                source_url: record.source_url.clone(),
                geometry: record.geometry.clone(),
                details: details.clone(),
            };
            if let Some(&position) = groups.get(&key) {
                let group = &mut out[position];
                group.details.retain(|k, v| details.get(k) == Some(v));
                group.segments.push(segment);
                group.segment_count = Some(group.segments.len());
                if record.id < group.record.id {
                    group.record = record;
                    group.links = links;
                }
            } else {
                groups.insert(key, out.len());
                out.push(Resource {
                    record,
                    details,
                    links,
                    segment_count: Some(1),
                    segments: vec![segment],
                });
            }
        } else {
            out.push(Resource {
                record,
                details,
                links,
                segment_count: None,
                segments: Vec::new(),
            });
        }
    }
    for group in &mut out {
        group.segments.sort_by_key(|s| s.id);
        // A representative line is not the geometry of the whole street.
        if group.segment_count.is_some() {
            group.record.geometry = None;
        }
    }
    out
}
pub fn page(records: Vec<Resource>, offset: usize, limit: usize) -> Vec<Resource> {
    records
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|mut r| {
            r.segments.clear();
            r
        })
        .collect()
}
