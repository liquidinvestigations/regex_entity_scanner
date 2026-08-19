//! The scanning pipeline.
//!
//! ```text
//! fragment ─► prefilter ─► validate ─► normalise ─► resolve ─► entities
//! ```
//!
//! `prefilter` and `resolve` have a module each. `validate` and `normalise` live inside the rules,
//! because both are per-type knowledge and separating them there would only mean passing a
//! half-checked value between two functions that always run together.

pub mod prefilter;
pub mod resolve;

use anyhow::Result;

use crate::data::VendoredData;
use crate::model::Entity;
use crate::rules::{self, Candidate, Rule};
use prefilter::Prefilter;

/// Compiled rules plus the vendored data their validators consult. Built once, shared by every
/// request; scanning takes `&self` and holds no mutable state.
pub struct Scanner {
    rules: Vec<Box<dyn Rule>>,
    prefilter: Prefilter,
    data: VendoredData,
}

impl Scanner {
    pub fn new(data: VendoredData) -> Result<Self> {
        let rules = rules::all();
        let prefilter = Prefilter::compile(&rules)?;
        Ok(Self {
            rules,
            prefilter,
            data,
        })
    }

    pub fn rule_ids(&self) -> Vec<&'static str> {
        self.rules.iter().map(|rule| rule.id()).collect()
    }

    pub fn data(&self) -> &VendoredData {
        &self.data
    }

    /// Scans one fragment. `base_offset` is the byte offset of the fragment's first byte in the
    /// source document and is added to every span, so callers that window a large document get
    /// offsets they can use against the original bytes without arithmetic of their own.
    pub fn scan(&self, fragment: &str, base_offset: usize) -> Vec<Entity> {
        let mut accepted = Vec::new();

        for (rule_index, span) in self.prefilter.candidates(fragment) {
            let rule = &self.rules[rule_index];
            let candidate = Candidate {
                fragment,
                start: span.0,
                end: span.1,
                data: &self.data,
            };
            let Some(verdict) = rule.validate(&candidate) else {
                continue;
            };
            accepted.push(Entity {
                entity_type: rule.entity_type(),
                start: verdict.start,
                end: verdict.end,
                text: fragment[verdict.start..verdict.end].to_string(),
                value: verdict.value,
                confidence: verdict.confidence,
                rule_id: rule.id().to_string(),
                flags: verdict.flags,
            });
        }

        let mut entities = resolve::resolve(accepted);
        for entity in &mut entities {
            entity.start += base_offset;
            entity.end += base_offset;
        }
        entities
    }
}
