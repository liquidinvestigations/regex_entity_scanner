//! The HTTP surface.
//!
//! Stateless by construction: the scanner is built once and shared, every request carries its own
//! fragment, and nothing is written anywhere. That is what makes the container disposable and
//! horizontally scalable, and it is worth keeping true.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::explain::{self, catalog, ExplainRequest, Explanation};
use crate::model::Entity;
use crate::rules::RULE_SET_VERSION;
use crate::scan::Scanner;

/// Everything a handler needs. The size limit lives here rather than in a global because it is an
/// operational parameter of this process, and a test builds a router with a small one.
pub struct AppState {
    pub scanner: Arc<Scanner>,
    /// The largest fragment this process will scan. Applied twice, deliberately: as the router's
    /// body limit, which is what protects memory because it rejects before buffering, and against
    /// the `text` field, which is what turns an oversized fragment into a precise error instead of
    /// a generic transport failure.
    pub max_body_bytes: usize,
}

/// The router's body limit is the fragment limit plus room for the JSON around it. Without the
/// allowance a fragment of exactly the advertised size could never be submitted, because the quotes
/// and the field names push its body over the line, and the precise error would be unreachable.
const JSON_ENVELOPE_ALLOWANCE: usize = 1_024;

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    /// The fragment to scan. Callers windowing a large document overlap their windows by at least
    /// the longest matchable entity, and deduplicate on `(type, start, end)` afterwards.
    pub text: String,
    /// Byte offset of the fragment's first byte within the source document.
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub entities: Vec<Entity>,
    /// Which rule set produced these entities, so a consumer can compute what a reindex has to
    /// cover instead of reprocessing everything.
    pub rule_set_version: u32,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub rules: usize,
    pub tlds: usize,
    pub rule_set_version: u32,
}

#[derive(Debug, Serialize)]
pub struct RulesResponse {
    pub rules: Vec<RuleSummary>,
    pub rule_set_version: u32,
}

/// Enough to populate a rule picker without pulling every catalogue entry.
#[derive(Debug, Serialize)]
pub struct RuleSummary {
    pub rule_id: &'static str,
    #[serde(rename = "type")]
    pub entity_type: crate::model::EntityType,
    pub title: &'static str,
    pub compiled: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn router(state: Arc<AppState>) -> Router {
    let max_body_bytes = state.max_body_bytes + JSON_ENVELOPE_ALLOWANCE;
    Router::new()
        .route("/health", get(health))
        .route("/rules", get(rules))
        .route("/rules/{rule_id}", get(rule))
        .route("/scan", post(scan))
        .route("/explain", post(explain_entity))
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        rules: state.scanner.rule_ids().len(),
        tlds: state.scanner.data().tld_count(),
        rule_set_version: RULE_SET_VERSION,
    })
}

async fn rules(State(state): State<Arc<AppState>>) -> Json<RulesResponse> {
    let compiled = state.scanner.rule_ids();
    Json(RulesResponse {
        rule_set_version: RULE_SET_VERSION,
        rules: catalog::all()
            .iter()
            .map(|doc| RuleSummary {
                rule_id: doc.rule_id,
                entity_type: doc.entity_type,
                title: doc.title,
                // A documented rule that is not compiled into this build is a real thing to know
                // about: the catalogue is the repository's knowledge, not this binary's inventory.
                compiled: compiled.contains(&doc.rule_id),
            })
            .collect(),
    })
}

/// The static documentation for one rule, with no match in hand — the same knowledge the explainer
/// builds a card from.
async fn rule(
    Path(rule_id): Path<String>,
) -> Result<Json<&'static catalog::RuleDoc>, (StatusCode, Json<ErrorResponse>)> {
    catalog::lookup(&rule_id).map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("no rule documented under {rule_id}"),
            }),
        )
    })
}

/// Takes an entity exactly as `/scan` returned it and answers with a card for the reader who
/// clicked it. An undocumented `rule_id` is a 404 rather than an empty card, so a client shows
/// nothing instead of showing an empty box.
async fn explain_entity(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExplainRequest>,
) -> Result<Json<Explanation>, (StatusCode, Json<ErrorResponse>)> {
    explain::explain(&request, state.scanner.data())
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("no rule documented under {}", request.rule_id),
                }),
            )
        })
}

/// A request that is too large is refused at two layers. The router's body limit is the one that
/// protects memory, because it rejects before the body is buffered; the check on `text` is the one
/// that gives a precise error instead of a generic one. Both answer in the same error shape, so a
/// client parses one thing.
async fn scan(
    State(state): State<Arc<AppState>>,
    request: Result<Json<ScanRequest>, JsonRejection>,
) -> Result<Json<ScanResponse>, (StatusCode, Json<ErrorResponse>)> {
    let Json(request) = request.map_err(|rejection| {
        (
            rejection.status(),
            Json(ErrorResponse {
                error: rejection.body_text(),
            }),
        )
    })?;
    if request.text.len() > state.max_body_bytes {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse {
                error: format!(
                    "the fragment is {} bytes and the limit is {}",
                    request.text.len(),
                    state.max_body_bytes
                ),
            }),
        ));
    }
    let entities = state.scanner.scan(&request.text, request.offset);
    Ok(Json(ScanResponse {
        entities,
        rule_set_version: RULE_SET_VERSION,
    }))
}
