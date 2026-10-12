use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub search_url: Option<String>,
    pub search_index: String,
    pub http: reqwest::Client,
}
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error":self.1}))).into_response()
    }
}
impl From<sqlx::Error> for ApiError {
    fn from(_: sqlx::Error) -> Self {
        Self(
            StatusCode::SERVICE_UNAVAILABLE,
            "Service temporarily unavailable.",
        )
    }
}
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Service {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    #[serde(default)]
    pub provider_name: String,
    pub jurisdiction: String,
    pub official_url: String,
    pub source_url: String,
    pub instructions: String,
    #[serde(default = "default_type")]
    pub record_type: String,
    #[serde(default)]
    pub source_key: String,
    #[serde(default)]
    pub source_record_id: String,
    #[serde(default = "empty_object", skip_serializing)]
    pub attributes: serde_json::Value,
    #[serde(default)]
    pub geometry: Option<serde_json::Value>,
}
fn default_type() -> String {
    "service".into()
}
fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}
pub const SELECT_SERVICES: &str = "SELECT s.id,s.name,s.description,s.category,p.name AS provider_name,p.jurisdiction,s.official_url,s.source_url,s.instructions,s.record_type,s.source_key,s.source_record_id,s.attributes,s.geometry FROM services s JOIN providers p ON p.id=s.provider_id";
#[derive(Deserialize, Default)]
pub struct Filters {
    pub q: Option<String>,
    pub category: Option<String>,
    #[serde(rename = "type")]
    pub record_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
#[derive(Serialize)]
pub struct ServiceList {
    pub items: Vec<crate::projection::Resource>,
    pub search_mode: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<&'static str>,
    pub total: usize,
    /// Total is exact for projected retrieved candidates, not global ANN recall.
    pub total_scope: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_candidate_limit: Option<usize>,
    pub limit: usize,
    pub offset: usize,
}
pub async fn services(
    State(state): State<AppState>,
    Query(filters): Query<Filters>,
) -> Result<Json<ServiceList>, ApiError> {
    let q = filters.q.unwrap_or_default();
    let q = q.trim();
    let category = filters.category.unwrap_or_default();
    let kind = filters.record_type.unwrap_or_else(|| "service".into());
    let limit = filters.limit.unwrap_or(50);
    let offset = filters.offset.unwrap_or(0);
    if q.chars().count() > 200
        || category.len() > 200
        || !(1..=100).contains(&limit)
        || offset > 100000
        || ![
            "service",
            "place",
            "route",
            "outage",
            "infrastructure",
            "all",
        ]
        .contains(&kind.as_str())
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Invalid query, type, category, limit (1–100), or offset (0–100000).",
        ));
    }
    if !q.is_empty()
        && let Some(url) = &state.search_url
    {
        let config = crate::search::EmbeddingConfig::from_env().ok();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            crate::search::search_catalog(
                &state.http,
                url,
                &state.search_index,
                config.as_ref(),
                q,
                &category,
                &kind,
            ),
        )
        .await;
        if let Ok(Ok((mut items, hybrid))) = result {
            // Search documents intentionally omit raw attributes. Hydrate only
            // these ranked IDs from PostgreSQL; no index mutation is required.
            if !items.is_empty() {
                let ids: Vec<_> = items.iter().map(|r| r.id).collect();
                let attrs: Vec<(Uuid, serde_json::Value)> =
                    sqlx::query_as("SELECT id,attributes FROM services WHERE id=ANY($1)")
                        .bind(ids)
                        .fetch_all(&state.pool)
                        .await?;
                let mut attrs: std::collections::HashMap<_, _> = attrs.into_iter().collect();
                for item in &mut items {
                    item.attributes = attrs.remove(&item.id).unwrap_or_else(empty_object);
                }
            }
            let items = crate::projection::project(items);
            let total = items.len();
            return Ok(Json(ServiceList {
                items: crate::projection::page(items, offset, limit),
                total,
                total_scope: if hybrid {
                    "lexical_plus_bounded_semantic_candidates"
                } else {
                    "lexical_matches"
                },
                semantic_candidate_limit: hybrid.then_some(crate::search::SEMANTIC_CANDIDATE_LIMIT),
                limit,
                offset,
                search_mode: if hybrid { "hybrid" } else { "keyword" },
                warning: if hybrid {
                    None
                } else {
                    Some(
                        "Keyword search only: embedding configuration, calibrated model, or vector retrieval unavailable.",
                    )
                },
            }));
        }
    }
    let predicate = " WHERE ($1 = '' OR strpos(lower(s.name || ' ' || s.description || ' ' || p.name || ' ' || s.category),lower($1)) > 0) AND ($2 = '' OR s.category=$2) AND ($3 = 'all' OR s.record_type=$3)";
    // Same snapshot for count and page, including empty/out-of-range pages.
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    // One bounded snapshot, one shared resident projection for browse and search.
    // Do not LIMIT/OFFSET raw street segments or count them as resources.
    let items = sqlx::query_as::<_, Service>(&format!(
        "{SELECT_SERVICES}{predicate} ORDER BY s.name,s.id LIMIT 100001"
    ))
    .bind(q)
    .bind(category)
    .bind(kind)
    .fetch_all(&mut *tx)
    .await?;
    if items.len() > 100000 {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Catalog projection bound exceeded.",
        ));
    }
    let items = crate::projection::project(items);
    let count = items.len();
    tx.commit().await?;
    Ok(Json(ServiceList {
        items: crate::projection::page(items, offset, limit),
        total: count,
        total_scope: "database_matches",
        semantic_candidate_limit: None,
        limit,
        offset,
        search_mode: "database",
        warning: if q.is_empty() {
            None
        } else {
            Some(
                "Database text search fallback: OpenSearch unavailable, incomplete, or not configured; semantic retrieval not used.",
            )
        },
    }))
}
pub async fn service(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<crate::projection::Resource>, ApiError> {
    let id = Uuid::parse_str(&id)
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "Invalid service ID."))?;
    let item = sqlx::query_as::<_, Service>(&format!("{SELECT_SERVICES} WHERE s.id=$1"))
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    let item = item.ok_or(ApiError(StatusCode::NOT_FOUND, "Service not found."))?;
    let records = if let Some(key) = crate::projection::street_key(&item) {
        // Any original segment UUID resolves to the complete street with provenance.
        sqlx::query_as::<_, Service>(&format!(
            "{SELECT_SERVICES} WHERE s.source_key=$1 AND s.record_type='route' ORDER BY s.id"
        ))
        .bind(&item.source_key)
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .filter(|s| crate::projection::street_key(s).as_ref() == Some(&key))
        .collect()
    } else {
        vec![item]
    };
    crate::projection::project(records)
        .into_iter()
        .next()
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND, "Service not found."))
}
pub async fn ticket(
    State(state): State<AppState>,
    payload: Result<Json<crate::TicketInput>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let Json(input) = payload.map_err(|rejection| {
        ApiError(
            rejection.status(),
            "Invalid ticket JSON or request body too large.",
        )
    })?;
    input
        .validate()
        .map_err(|message| ApiError(StatusCode::BAD_REQUEST, message))?;
    let service_id = input
        .service_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "Invalid service ID."))?;
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO tickets (id,service_id,description,contact_email,status,demo) VALUES ($1,$2,$3,$4,'new',true)")
        .bind(id).bind(service_id).bind(input.description.trim()).bind(input.contact_email).execute(&state.pool).await;
    if let Err(error) = result {
        if error
            .as_database_error()
            .is_some_and(|e| e.is_foreign_key_violation())
        {
            return Err(ApiError(StatusCode::BAD_REQUEST, "Service not found."));
        }
        return Err(error.into());
    }
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"id":id,"status":"new","demo":true})),
    ))
}
pub fn router(pool: PgPool) -> Router {
    router_with_search(pool, None, "manassas-catalog".into())
}
pub fn router_with_search(
    pool: PgPool,
    search_url: Option<String>,
    search_index: String,
) -> Router {
    Router::new()
        .route("/api/tickets", axum::routing::post(ticket))
        .layer(axum::extract::DefaultBodyLimit::max(16384))
        .route("/api/services", get(services))
        .route("/api/services/{id}", get(service))
        .route("/api/map/features", get(crate::map::features))
        .route("/api/map/features/{id}", get(crate::map::detail))
        .layer(axum::Extension(crate::map::MapCache::default()))
        .with_state(AppState {
            pool,
            search_url,
            search_index,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .expect("HTTP client"),
        })
}
