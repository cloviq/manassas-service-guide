use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
fn exact_filters(category: &str, kind: &str) -> Vec<Value> {
    let mut f = vec![];
    if !category.is_empty() {
        f.push(json!({"term":{"category.raw":category}}));
    }
    if kind != "all" {
        f.push(json!({"term":{"record_type":kind}}));
    }
    f
}
pub fn lexical_query(q: &str, category: &str, kind: &str) -> Value {
    let mut v = keyword_query(q, category);
    v["query"]["bool"]["filter"] = json!(exact_filters(category, kind));
    v["size"] = json!(1000);
    v["track_total_hits"] = json!(true);
    v["_source"] = json!({"excludes":["embedding"]});
    v["sort"] = json!([{"_score":"desc"},{"id":"asc"}]);
    v
}
// Calibrated for BAAI/bge-small-en-v1.5 on this catalog. See SEARCH-RELEVANCE.md.
// This is an abstention boundary, not a probability of relevance.
pub const MIN_SEMANTIC_COSINE: f64 = 0.615;
pub const SEMANTIC_CANDIDATE_LIMIT: usize = 100;
pub fn vector_query(vector: &[f32], category: &str, kind: &str) -> Value {
    // Existing Lucene cosine mapping scores (1 + cosine)/2, verified against
    // direct dot products. Filter inside ANN, never scroll the entire catalog.
    json!({"size":SEMANTIC_CANDIDATE_LIMIT,"track_total_hits":true,
        "min_score":(1.0 + MIN_SEMANTIC_COSINE)/2.0,
        "_source":{"excludes":["embedding"]},
        "sort":[{"_score":"desc"},{"id":"asc"}],
        "query":{"knn":{"embedding":{"vector":vector,"k":SEMANTIC_CANDIDATE_LIMIT,
            "filter":{"bool":{"filter":exact_filters(category,kind)}}}}}})
}
pub fn rrf_ids(a: &[String], b: &[String]) -> Vec<String> {
    let mut scores = BTreeMap::<String, f64>::new();
    for leg in [a, b] {
        for (i, id) in leg.iter().enumerate() {
            *scores.entry(id.clone()).or_default() += 1.0 / (60.0 + i as f64 + 1.0);
        }
    }
    let mut ranks: Vec<_> = scores.into_iter().collect();
    ranks.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    ranks.into_iter().map(|(id, _)| id).collect()
}
// Search-time equivalence groups for everyday words the City text does not use
// ("garbage" vs "trash"). Lexical leg only: the calibrated semantic gate above is
// unchanged, and synonyms only widen exact AND matching. Requires a reindex.
pub const CIVIC_SYNONYMS: &[&str] = &[
    "garbage, trash, refuse, rubbish",
    "bin, can, cart, container, toter",
    "recycle, recycling, recyclables",
    "pickup, pick up, collection",
    "leaf, leaves",
    "couch, sofa",
    "plow, plowing, plowed",
];
// Bump whenever the mapping, analyzers or indexed document shape change so
// `index --if-stale` (run by `make up`) rebuilds existing indexes.
pub const SEARCH_SCHEMA_VERSION: u64 = 2;
pub fn index_mapping(dimension: usize, model: &str) -> Value {
    json!({"settings":{"number_of_shards":1,"number_of_replicas":0,"index.knn":true,
        "analysis":{"filter":{"civic_synonyms":{"type":"synonym_graph","synonyms":CIVIC_SYNONYMS}},
            "analyzer":{"civic_search":{"tokenizer":"standard","filter":["lowercase","civic_synonyms"]}}}},"mappings":{"_meta":{"search_schema":SEARCH_SCHEMA_VERSION,"embedding_model":model,"embedding_dimension":dimension,"similarity":"cosine","fusion":"rrf-k60"},"properties":{"id":{"type":"keyword"},"record_type":{"type":"keyword"},"source_key":{"type":"keyword"},"attributes":{"type":"object","enabled":false},"geometry":{"type":"object","enabled":false},"category":{"type":"text","fields":{"raw":{"type":"keyword"}}},"embedding":{"type":"knn_vector","dimension":dimension,"method":{"name":"hnsw","space_type":"cosinesimil","engine":"lucene","parameters":{"ef_construction":128,"m":16}}}}}})
}

pub fn keyword_query(q: &str, category: &str) -> Value {
    let mut filters = vec![];
    if !category.is_empty() {
        filters.push(json!({"term":{"category.raw":category}}));
    }
    json!({"size":100,"query":{"bool":{"must":[{"multi_match":{"query":q,"fields":["name^3","description","provider_name","category"],"type":"best_fields","operator":"and","analyzer":"civic_search"}}],"filter":filters}}})
}
pub fn decode_hits(value: Value) -> Result<Vec<crate::api::Service>, &'static str> {
    let hits = value["hits"]["hits"]
        .as_array()
        .ok_or("Invalid search response")?;
    hits.iter()
        .map(|hit| {
            serde_json::from_value(hit["_source"].clone()).map_err(|_| "Invalid search document")
        })
        .collect()
}

pub async fn index_documents(
    url: &str,
    index: &str,
    documents: &[crate::api::Service],
) -> Result<usize, &'static str> {
    if index.is_empty()
        || !index
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
    {
        return Err("Invalid search index name.");
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "HTTP client failed.")?;
    let base = format!("{}/{}", url.trim_end_matches('/'), index);
    let response = client.put(&base).json(&json!({
        "settings":{"number_of_shards":1,"number_of_replicas":0},
        "mappings":{"properties":{"category":{"type":"text","fields":{"raw":{"type":"keyword"}}}}}
    })).send().await.map_err(|_| "Index creation request failed.")?;
    if !response.status().is_success() {
        let body: Value = response
            .json()
            .await
            .map_err(|_| "Index creation failed.")?;
        if body["error"]["type"] != "resource_already_exists_exception" {
            return Err("Index creation failed.");
        }
    }
    let mut bulk = String::new();
    for document in documents {
        bulk.push_str(&json!({"index":{"_id":document.id}}).to_string());
        bulk.push('\n');
        bulk.push_str(
            &serde_json::to_string(document).map_err(|_| "Document serialization failed.")?,
        );
        bulk.push('\n');
    }
    if !bulk.is_empty() {
        let result: Value = client
            .post(format!("{base}/_bulk?refresh=true"))
            .header("content-type", "application/x-ndjson")
            .body(bulk)
            .send()
            .await
            .map_err(|_| "Bulk indexing request failed.")?
            .error_for_status()
            .map_err(|_| "Bulk indexing failed.")?
            .json()
            .await
            .map_err(|_| "Invalid bulk response.")?;
        if result["errors"] != false {
            return Err("One or more documents failed indexing.");
        }
    }
    let count: Value = client
        .get(format!("{base}/_count"))
        .send()
        .await
        .map_err(|_| "Index readback failed.")?
        .error_for_status()
        .map_err(|_| "Index readback failed.")?
        .json()
        .await
        .map_err(|_| "Invalid index readback.")?;
    count["count"]
        .as_u64()
        .map(|n| n as usize)
        .ok_or("Invalid index count.")
}

#[cfg(test)]
mod tests {
    #[test]
    fn valid_empty_search_is_not_a_backend_failure() {
        assert!(super::decode_hits(serde_json::json!({"hits":{"hits":[]}})).is_ok());
        assert!(super::decode_hits(serde_json::json!({"error":"unavailable"})).is_err());
        assert!(super::decode_hits(serde_json::json!({"hits":{"hits":[{"_source":{}}]}})).is_err());
    }
    use super::*;
    #[test]
    fn query_searches_all_contract_fields_with_exact_category_filter() {
        let query = keyword_query("recycling", "Trash & recycling");
        assert_eq!(
            query["query"]["bool"]["must"][0]["multi_match"]["query"],
            "recycling"
        );
        assert_eq!(
            query["query"]["bool"]["must"][0]["multi_match"]["fields"],
            json!(["name^3", "description", "provider_name", "category"])
        );
        assert_eq!(
            query["query"]["bool"]["filter"][0]["term"]["category.raw"],
            "Trash & recycling"
        );
        assert!(!query.to_string().contains("knn"));
        assert_eq!(
            query["query"]["bool"]["must"][0]["multi_match"]["analyzer"],
            "civic_search"
        );
    }
}

#[derive(Clone)]
pub struct EmbeddingConfig {
    pub url: String,
    pub model: String,
    pub dimension: usize,
    pub batch_size: usize,
}
impl EmbeddingConfig {
    pub fn from_env() -> Result<Self, &'static str> {
        let dimension = std::env::var("EMBEDDING_DIMENSION")
            .unwrap_or_else(|_| "384".into())
            .parse()
            .map_err(|_| "Invalid EMBEDDING_DIMENSION")?;
        let batch_size = std::env::var("EMBEDDING_BATCH_SIZE")
            .unwrap_or_else(|_| "32".into())
            .parse()
            .map_err(|_| "Invalid EMBEDDING_BATCH_SIZE")?;
        if !(1..=4096).contains(&dimension) || !(1..=128).contains(&batch_size) {
            return Err("Embedding dimension/batch out of bounds");
        }
        Ok(Self {
            url: std::env::var("EMBEDDING_URL").map_err(|_| "EMBEDDING_URL not configured")?,
            model: std::env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "BAAI/bge-small-en-v1.5".into()),
            dimension,
            batch_size,
        })
    }
    pub async fn embed(
        &self,
        client: &reqwest::Client,
        input: &[String],
    ) -> Result<Vec<Vec<f32>>, &'static str> {
        if input.is_empty() || input.len() > 128 || input.iter().any(|s| s.len() > 16000) {
            return Err("Embedding input exceeds bounds");
        }
        let v = request_json(
            client
                .post(&self.url)
                .timeout(std::time::Duration::from_secs(60))
                .json(&json!({"model":self.model,"input":input})),
        )
        .await?;
        if v["model"].as_str() != Some(&self.model) {
            return Err("Embedding model mismatch");
        }
        let data = v["data"].as_array().ok_or("Invalid embedding data")?;
        if data.len() != input.len() {
            return Err("Embedding count mismatch");
        }
        let mut out = vec![None; input.len()];
        for row in data {
            let i = row["index"].as_u64().ok_or("Missing embedding index")? as usize;
            if i >= out.len() || out[i].is_some() {
                return Err("Invalid embedding index");
            }
            let vector: Vec<f32> = serde_json::from_value(row["embedding"].clone())
                .map_err(|_| "Invalid embedding vector")?;
            if vector.len() != self.dimension
                || vector.iter().any(|v| !v.is_finite())
                || vector.iter().all(|v| *v == 0.0)
            {
                return Err("Embedding dimension or norm invalid");
            }
            out[i] = Some(vector);
        }
        out.into_iter()
            .map(|v| v.ok_or("Missing embedding"))
            .collect()
    }
}
async fn request_json(req: reqwest::RequestBuilder) -> Result<Value, &'static str> {
    let response = req
        .send()
        .await
        .map_err(|_| "Search/model request failed")?
        .error_for_status()
        .map_err(|_| "Search/model HTTP failure")?;
    if response
        .content_length()
        .is_some_and(|n| n > 64 * 1024 * 1024)
    {
        return Err("Search/model response too large");
    }
    response
        .json()
        .await
        .map_err(|_| "Invalid search/model JSON")
}
async fn retrieve(
    client: &reqwest::Client,
    url: &str,
    index: &str,
    query: Value,
) -> Result<Vec<crate::api::Service>, &'static str> {
    let mut v = request_json(
        client
            .post(format!(
                "{}/{index}/_search?scroll=1m",
                url.trim_end_matches('/')
            ))
            .json(&query),
    )
    .await?;
    let mut out = vec![];
    let mut scroll = None;
    let result = async {
        for _ in 0..101 {
            if v["timed_out"] == true || v["_shards"]["failed"].as_u64().unwrap_or(0) > 0 {
                return Err("Incomplete search response");
            }
            if let Some(id) = v["_scroll_id"].as_str() {
                scroll = Some(id.to_owned());
            }
            let batch = decode_hits(v.clone())?;
            let n = batch.len();
            out.extend(batch);
            if out.len() > 100000 {
                return Err("Search result bound exceeded");
            }
            let total = v["hits"]["total"]["value"]
                .as_u64()
                .ok_or("Missing exact search total")?;
            if v["hits"]["total"]["relation"] != "eq" {
                return Err("Nonexact search total");
            }
            if out.len() as u64 == total {
                return Ok(());
            }
            if n == 0 || out.len() as u64 > total {
                return Err("Incomplete search page");
            }
            let id = scroll.as_deref().ok_or("Incomplete search page")?;
            v = request_json(
                client
                    .post(format!("{}/_search/scroll", url.trim_end_matches('/')))
                    .json(&json!({"scroll":"1m","scroll_id":id})),
            )
            .await?;
        }
        Err("Search page bound exceeded")
    }
    .await;
    if let Some(id) = scroll {
        let _ = client
            .delete(format!("{}/_search/scroll", url.trim_end_matches('/')))
            .json(&json!({"scroll_id":[id]}))
            .send()
            .await;
    }
    result?;
    Ok(out)
}
pub async fn search_catalog(
    client: &reqwest::Client,
    url: &str,
    index: &str,
    embedding: Option<&EmbeddingConfig>,
    q: &str,
    category: &str,
    kind: &str,
) -> Result<(Vec<crate::api::Service>, bool), &'static str> {
    let lexical = retrieve(client, url, index, lexical_query(q, category, kind)).await?;
    let semantic = async {
        let config = embedding.ok_or("Embeddings disabled")?;
        if config.model != "BAAI/bge-small-en-v1.5" || config.dimension != 384 {
            return Err("Model has no calibrated relevance boundary");
        }
        let vectors = config.embed(client, &[q.to_owned()]).await?;
        let response = request_json(
            client
                .post(format!("{}/{index}/_search", url.trim_end_matches('/')))
                .json(&vector_query(&vectors[0], category, kind)),
        )
        .await?;
        if response["timed_out"] == true || response["_shards"]["failed"].as_u64().unwrap_or(0) > 0
        {
            return Err("Incomplete semantic candidates");
        }
        let total = response["hits"]["total"]["value"]
            .as_u64()
            .ok_or("Missing semantic total")?;
        if response["hits"]["total"]["relation"] != "eq" {
            return Err("Nonexact semantic candidate total");
        }
        let candidates = decode_hits(response)?;
        if total != candidates.len() as u64 {
            return Err("Incomplete semantic candidates");
        }
        if candidates.len() > SEMANTIC_CANDIDATE_LIMIT {
            return Err("Semantic candidate bound exceeded");
        }
        Ok(candidates)
    }
    .await;
    let Ok(vector) = semantic else {
        return Ok((lexical, false));
    };
    let a: Vec<_> = lexical.iter().map(|d| d.id.to_string()).collect();
    let b: Vec<_> = vector.iter().map(|d| d.id.to_string()).collect();
    let ranks = rrf_ids(&a, &b);
    let mut docs: HashMap<_, _> = lexical
        .into_iter()
        .chain(vector)
        .map(|d| (d.id.to_string(), d))
        .collect();
    Ok((
        ranks
            .into_iter()
            .filter_map(|id| docs.remove(&id))
            .collect(),
        true,
    ))
}

pub async fn publish_index(
    client: &reqwest::Client,
    url: &str,
    alias: &str,
    physical: &str,
    expected: usize,
) -> Result<(), &'static str> {
    let count = request_json(client.get(format!("{url}/{physical}/_count"))).await?;
    if count["count"].as_u64() != Some(expected as u64) {
        return Err("Index count mismatch; alias unchanged");
    }
    let response = client
        .get(format!("{url}/_alias/{alias}"))
        .send()
        .await
        .map_err(|_| "Alias lookup failed")?;
    let mut actions = vec![];
    let mut superseded = vec![];
    if response.status().is_success() {
        let aliases: Value = response
            .json()
            .await
            .map_err(|_| "Invalid alias response")?;
        for old in aliases.as_object().ok_or("Invalid alias response")?.keys() {
            actions.push(json!({"remove":{"index":old,"alias":alias}}));
            superseded.push(old.clone());
        }
    } else if response.status() != reqwest::StatusCode::NOT_FOUND {
        return Err("Alias lookup failed");
    }
    actions.push(json!({"add":{"index":physical,"alias":alias}}));
    let result = request_json(
        client
            .post(format!("{url}/_aliases"))
            .json(&json!({"actions":actions})),
    )
    .await?;
    if result["acknowledged"] != true {
        return Err("Alias switch not acknowledged");
    }
    let readback = request_json(client.get(format!("{url}/_alias/{alias}"))).await?;
    if readback
        .as_object()
        .is_none_or(|m| m.len() != 1 || !m.contains_key(physical))
    {
        return Err("Alias readback mismatch");
    }
    // The alias now serves `physical`; drop the indexes it replaced so repeated
    // reindexing does not grow disk use. Best effort: a leftover is harmless.
    for old in superseded.iter().filter(|old| old.as_str() != physical) {
        let _ = client.delete(format!("{url}/{old}")).send().await;
    }
    Ok(())
}
/// True when `alias` already serves the current schema and model with exactly
/// one document per database record; anything else (missing alias, older
/// schema, count drift) means a rebuild is needed.
pub async fn index_is_current(
    pool: &sqlx::PgPool,
    url: &str,
    alias: &str,
    config: &EmbeddingConfig,
) -> Result<bool, &'static str> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "HTTP client failed")?;
    let url = url.trim_end_matches('/');
    let response = client
        .get(format!("{url}/{alias}/_mapping"))
        .send()
        .await
        .map_err(|_| "Search mapping lookup failed")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }
    if !response.status().is_success() {
        return Err("Search mapping lookup failed");
    }
    let mapping: Value = response
        .json()
        .await
        .map_err(|_| "Invalid mapping response")?;
    let indexes = mapping.as_object().ok_or("Invalid mapping response")?;
    let [(_, physical)] = indexes.iter().collect::<Vec<_>>()[..] else {
        return Ok(false);
    };
    let meta = &physical["mappings"]["_meta"];
    if meta["search_schema"].as_u64() != Some(SEARCH_SCHEMA_VERSION)
        || meta["embedding_model"].as_str() != Some(&config.model)
        || meta["embedding_dimension"].as_u64() != Some(config.dimension as u64)
    {
        return Ok(false);
    }
    let indexed = request_json(client.get(format!("{url}/{alias}/_count"))).await?["count"]
        .as_u64()
        .ok_or("Invalid count response")?;
    let expected: i64 = sqlx::query_scalar("SELECT count(*) FROM services")
        .fetch_one(pool)
        .await
        .map_err(|_| "Catalog count failed")?;
    Ok(indexed == expected as u64)
}
pub async fn index_catalog(
    pool: &sqlx::PgPool,
    url: &str,
    alias: &str,
    config: &EmbeddingConfig,
) -> Result<usize, &'static str> {
    if alias.is_empty()
        || alias.len() > 100
        || !alias
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
    {
        return Err("Invalid alias name");
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|_| "HTTP client failed")?;
    let url = url.trim_end_matches('/');
    let physical = format!("{alias}-{}", uuid::Uuid::new_v4().simple());
    let mut tx = pool.begin().await.map_err(|_| "Index snapshot failed")?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| "Index snapshot setup failed")?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *tx)
        .await
        .map_err(|_| "Index timeout setup failed")?;
    let expected: i64 = sqlx::query_scalar("SELECT count(*) FROM services")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Catalog count failed")?;
    if !(1..=100000).contains(&expected) {
        return Err("Index catalog bound is 1–100000");
    }
    request_json(
        client
            .put(format!("{url}/{physical}"))
            .json(&index_mapping(config.dimension, &config.model)),
    )
    .await?;
    let mut cursor = uuid::Uuid::nil();
    let mut indexed = 0;
    loop {
        let docs = sqlx::query_as::<_, crate::api::Service>(&format!(
            "{} WHERE s.id>$1 ORDER BY s.id LIMIT $2",
            crate::api::SELECT_SERVICES
        ))
        .bind(cursor)
        .bind(config.batch_size as i64)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| "Catalog page failed")?;
        if docs.is_empty() {
            break;
        }
        let input: Vec<String> = docs
            .iter()
            .map(|d| {
                format!("{}\n{}", d.name, d.description)
                    .chars()
                    .take(3500)
                    .collect()
            })
            .collect();
        let vectors = config.embed(&client, &input).await?;
        let mut bulk = String::new();
        for (doc, vector) in docs.iter().zip(vectors) {
            bulk.push_str(&json!({"index":{"_id":doc.id}}).to_string());
            bulk.push('\n');
            let mut value = serde_json::to_value(doc).map_err(|_| "Document encode failed")?;
            value["embedding"] = json!(vector);
            bulk.push_str(&value.to_string());
            bulk.push('\n');
            cursor = doc.id;
        }
        let response = request_json(
            client
                .post(format!("{url}/{physical}/_bulk"))
                .header("content-type", "application/x-ndjson")
                .body(bulk),
        )
        .await?;
        if response["errors"] != false {
            return Err("Bulk index error; alias unchanged");
        }
        indexed += docs.len();
        if indexed > 100000 {
            return Err("Index bound exceeded");
        }
    }
    tx.commit()
        .await
        .map_err(|_| "Index snapshot completion failed")?;
    if indexed != expected as usize {
        return Err("Index snapshot count mismatch");
    }
    request_json(client.post(format!("{url}/{physical}/_refresh"))).await?;
    publish_index(&client, url, alias, &physical, indexed).await?;
    Ok(indexed)
}
