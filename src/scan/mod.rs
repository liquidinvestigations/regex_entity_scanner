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

use std::collections::HashSet;

use anyhow::Result;

use crate::data::VendoredData;
use crate::model::Entity;
use crate::rules::{self, Candidate, Rule};
use prefilter::Prefilter;

/// How many times one prefilter candidate may be shrunk in the search for a valid match nested
/// inside it. Unbounded retry is quadratic in the fragment on adversarial input, which is the
/// property the linear-time prefilter exists to protect, so the recovery is best-effort by
/// construction.
const RETRY_PER_CANDIDATE: usize = 8;

/// The same bound across the whole fragment, so a fragment full of near-miss candidates cannot buy
/// unbounded work a few retries at a time.
const RETRY_PER_FRAGMENT: usize = 256;

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
    ///
    /// A rejected candidate is not the end of the story. The prefilter takes leftmost-longest per
    /// rule, so a long over-matched span that fails its validator routinely contains a shorter one
    /// that would pass — an identifier-shaped run whose tail is a page number, an address whose
    /// first reading ends in a top-level domain that does not exist. Rejection therefore re-runs
    /// the rule's own pattern over the candidate's interior and queues whatever it finds, bounded
    /// by [`RETRY_PER_CANDIDATE`] and [`RETRY_PER_FRAGMENT`].
    pub fn scan(&self, fragment: &str, base_offset: usize) -> Vec<Entity> {
        let mut accepted = Vec::new();
        let mut visited: HashSet<(usize, usize, usize)> = HashSet::new();
        let mut fragment_retries = 0usize;

        // `(rule index, span, how many times this candidate's lineage has already been shrunk)`.
        let mut queue: Vec<(usize, (usize, usize), usize)> = self
            .prefilter
            .candidates(fragment)
            .into_iter()
            .map(|(rule_index, span)| (rule_index, span, 0))
            .collect();

        while let Some((rule_index, (start, end), retries)) = queue.pop() {
            if !visited.insert((rule_index, start, end)) {
                continue;
            }

            let rule = &self.rules[rule_index];
            // The validator sees the whole fragment and absolute offsets even on a retry, so the
            // adjacent-byte guards read the real neighbours rather than a slice's edges.
            let candidate = Candidate {
                fragment,
                start,
                end,
                data: &self.data,
            };
            if let Some(verdict) = rule.validate(&candidate) {
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
                continue;
            }

            if retries >= RETRY_PER_CANDIDATE || fragment_retries >= RETRY_PER_FRAGMENT {
                continue;
            }
            let Some(interior_end) = previous_boundary(fragment, start, end) else {
                continue;
            };
            fragment_retries += 1;

            // Shrinking the window by one character from the right and re-running the rule's own
            // pattern over the interior yields both cases in one mechanism: the shorter match at
            // the same start, and the nested match that starts later.
            for (inner_start, inner_end) in self
                .prefilter
                .find_in(rule_index, &fragment[start..interior_end])
            {
                queue.push((
                    rule_index,
                    (start + inner_start, start + inner_end),
                    retries + 1,
                ));
            }
        }

        let mut entities = resolve::resolve(accepted);
        for entity in &mut entities {
            entity.start += base_offset;
            entity.end += base_offset;
        }
        entities
    }
}

/// The largest character boundary strictly below `end`, or `None` when that would not leave a
/// non-empty span starting at `start`.
fn previous_boundary(fragment: &str, start: usize, end: usize) -> Option<usize> {
    let mut boundary = end.checked_sub(1)?;
    while boundary > start && !fragment.is_char_boundary(boundary) {
        boundary -= 1;
    }
    (boundary > start).then_some(boundary)
}
