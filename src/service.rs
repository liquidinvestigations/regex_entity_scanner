//! The HTTP surface.
//!
//! Stateless by construction: the scanner is built once and shared, every request carries its own
//! fragment, and nothing is written anywhere. That is what makes the container disposable and
//! horizontally scalable, and it is worth keeping true.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::explain::{self, catalog, ExplainRequest, Explanation};
use crate::model::Entity;
use crate::scan::Scanner;

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
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub rules: usize,
    pub tlds: usize,
}

#[derive(Debug, Serialize)]
pub struct RulesResponse {
    pub rules: Vec<RuleSummary>,
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

pub fn router(scanner: Arc<Scanner>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/rules", get(rules))
        .route("/rules/{rule_id}", get(rule))
        .route("/scan", post(scan))
        .route("/explain", post(explain_entity))
        .with_state(scanner)
}

async fn health(State(scanner): State<Arc<Scanner>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        rules: scanner.rule_ids().len(),
        tlds: scanner.data().tld_count(),
    })
}

async fn rules(State(scanner): State<Arc<Scanner>>) -> Json<RulesResponse> {
    let compiled = scanner.rule_ids();
    Json(RulesResponse {
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
    State(scanner): State<Arc<Scanner>>,
    Json(request): Json<ExplainRequest>,
) -> Result<Json<Explanation>, (StatusCode, Json<ErrorResponse>)> {
    explain::explain(&request, scanner.data())
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

async fn scan(
    State(scanner): State<Arc<Scanner>>,
    Json(request): Json<ScanRequest>,
) -> (StatusCode, Json<ScanResponse>) {
    let entities = scanner.scan(&request.text, request.offset);
    (StatusCode::OK, Json(ScanResponse { entities }))
}
