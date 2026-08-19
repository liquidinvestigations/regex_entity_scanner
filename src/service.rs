//! The HTTP surface.
//!
//! Stateless by construction: the scanner is built once and shared, every request carries its own
//! fragment, and nothing is written anywhere. That is what makes the container disposable and
//! horizontally scalable, and it is worth keeping true.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

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
    pub rules: Vec<&'static str>,
}

pub fn router(scanner: Arc<Scanner>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/rules", get(rules))
        .route("/scan", post(scan))
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
    Json(RulesResponse {
        rules: scanner.rule_ids(),
    })
}

async fn scan(
    State(scanner): State<Arc<Scanner>>,
    Json(request): Json<ScanRequest>,
) -> (StatusCode, Json<ScanResponse>) {
    let entities = scanner.scan(&request.text, request.offset);
    (StatusCode::OK, Json(ScanResponse { entities }))
}
