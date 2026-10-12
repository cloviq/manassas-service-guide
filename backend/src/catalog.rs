use crate::api::Service;
use std::collections::{HashMap, HashSet};
pub fn parse_catalog(bytes: &[u8]) -> Result<Vec<Service>, &'static str> {
    if bytes.len() > 128 * 1024 * 1024 {
        return Err("Catalog exceeds 128 MiB");
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Invalid catalog JSON")?;
    let rows = if value.is_array() {
        value
    } else {
        value
            .get("records")
            .cloned()
            .ok_or("Expected array or records array")?
    };
    let mut docs: Vec<Service> =
        serde_json::from_value(rows).map_err(|_| "Catalog record does not match contract")?;
    if docs.is_empty() || docs.len() > 100000 {
        return Err("Catalog must contain 1–100000 records");
    }
    let mut ids = HashSet::new();
    for d in &mut docs {
        if !ids.insert(d.id) {
            return Err("Duplicate catalog ID");
        }
        if d.name.trim().is_empty()
            || d.name.len() > 1000
            || d.description.len() > 16000
            || d.instructions.len() > 16000
            || d.category.len() > 200
            || d.source_key.is_empty()
            || d.source_record_id.is_empty()
            || !d.attributes.is_object()
            || d.geometry.as_ref().is_some_and(|g| !g.is_object())
            || !["service", "place", "route", "outage", "infrastructure"]
                .contains(&d.record_type.as_str())
        {
            return Err("Invalid or oversized catalog record");
        }
        for url in [&d.official_url, &d.source_url] {
            if url.len() > 4000 || !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err("Invalid source URL");
            }
        }
        // Publisher/source metadata is never promoted into operator attribution.
        if d.provider_name.trim().is_empty() {
            d.provider_name = "Operator not verified".into();
        }
    }
    Ok(docs)
}
pub async fn validate_catalog_file(path: &str) -> Result<Vec<Service>, &'static str> {
    let metadata = tokio::fs::metadata(path)
        .await
        .map_err(|_| "Cannot read catalog")?;
    if metadata.len() > 128 * 1024 * 1024 {
        return Err("Catalog exceeds 128 MiB");
    }
    parse_catalog(
        &tokio::fs::read(path)
            .await
            .map_err(|_| "Cannot read catalog")?,
    )
}

pub async fn import_catalog(pool: &sqlx::PgPool, path: &str) -> Result<usize, &'static str> {
    let docs = validate_catalog_file(path).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| "Import transaction failed")?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| "Import timeout setup failed")?;
    let mut providers = HashMap::new();
    for d in &docs {
        let key = (d.provider_name.clone(), d.jurisdiction.clone());
        let provider = if let Some(id) = providers.get(&key) {
            *id
        } else {
            let existing: Option<uuid::Uuid> = sqlx::query_scalar(
                "SELECT id FROM providers WHERE name=$1 AND jurisdiction=$2 ORDER BY id LIMIT 1",
            )
            .bind(&d.provider_name)
            .bind(&d.jurisdiction)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| "Provider lookup failed")?;
            let id = existing.unwrap_or_else(uuid::Uuid::new_v4);
            if existing.is_none() {
                sqlx::query(
                    "INSERT INTO providers(id,name,jurisdiction,official_url) VALUES($1,$2,$3,$4)",
                )
                .bind(id)
                .bind(&d.provider_name)
                .bind(&d.jurisdiction)
                .bind(&d.official_url)
                .execute(&mut *tx)
                .await
                .map_err(|_| "Provider import failed")?;
            }
            providers.insert(key, id);
            id
        };
        sqlx::query("INSERT INTO services(id,provider_id,name,description,category,official_url,source_url,instructions,record_type,source_key,source_record_id,attributes,geometry) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) ON CONFLICT(id) DO UPDATE SET provider_id=excluded.provider_id,name=excluded.name,description=excluded.description,category=excluded.category,official_url=excluded.official_url,source_url=excluded.source_url,instructions=excluded.instructions,record_type=excluded.record_type,source_key=excluded.source_key,source_record_id=excluded.source_record_id,attributes=excluded.attributes,geometry=excluded.geometry WHERE services.source_key <> 'seed'")
   .bind(d.id).bind(provider).bind(&d.name).bind(&d.description).bind(&d.category).bind(&d.official_url).bind(&d.source_url).bind(&d.instructions).bind(&d.record_type).bind(&d.source_key).bind(&d.source_record_id).bind(&d.attributes).bind(&d.geometry).execute(&mut *tx).await.map_err(|_|"Record import failed; transaction rolled back")?;
    }
    tx.commit().await.map_err(|_| "Import commit failed")?;
    let ids: Vec<_> = docs.iter().map(|d| d.id).collect();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM services WHERE id=ANY($1)")
        .bind(ids)
        .fetch_one(pool)
        .await
        .map_err(|_| "Import readback failed")?;
    if count as usize != docs.len() {
        return Err("Import readback count mismatch");
    }
    Ok(docs.len())
}
