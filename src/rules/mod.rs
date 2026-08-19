//! Rule definitions: one candidate pattern plus one validator each.
//!
//! A rule owns both halves of the two-stage design. [`Rule::candidate_pattern`] is deliberately
//! loose and must be expressible in a linear-time engine — no lookaround, no backreferences —
//! because it runs over raw, attacker-influenceable document text. [`Rule::validate`] then sees
//! one candidate at a time and may be as expensive as it likes: it re-imposes the guards that were
//! stripped out of the pattern, applies the checks that decide truth, and produces the canonical
//! value.
//!
//! Adding a type is therefore a pattern, a validator and a fixture — not a new pipeline.

pub mod date_iso;
pub mod email;

use crate::data::VendoredData;
use crate::model::{EntityType, Flag, Value};

/// A span the prefilter proposed, with the surrounding fragment available for the guard checks.
pub struct Candidate<'a> {
    /// The whole fragment, so a validator can look at what precedes and follows the span.
    pub fragment: &'a str,
    /// Byte offsets of the candidate within `fragment`.
    pub start: usize,
    pub end: usize,
    pub data: &'a VendoredData,
}

impl<'a> Candidate<'a> {
    pub fn text(&self) -> &'a str {
        &self.fragment[self.start..self.end]
    }

    /// The byte immediately before the candidate, if any. This is how a stripped `(?<!\d)` guard
    /// is re-imposed: the assertion is a statement about one neighbouring byte, and checking it
    /// here keeps the scanning pass linear.
    pub fn byte_before(&self) -> Option<u8> {
        self.fragment
            .as_bytes()
            .get(self.start.checked_sub(1)?)
            .copied()
    }

    /// The byte immediately after the candidate, if any.
    pub fn byte_after(&self) -> Option<u8> {
        self.fragment.as_bytes().get(self.end).copied()
    }
}

/// What a validator returns when it accepts a candidate.
pub struct Verdict {
    /// Offsets within the fragment. A validator may narrow the span it was given — trailing
    /// punctuation that the pattern swallowed is trimmed here, not in the pattern.
    pub start: usize,
    pub end: usize,
    pub value: Value,
    pub confidence: f32,
    pub flags: Vec<Flag>,
}

pub trait Rule: Send + Sync {
    /// Stable identifier, dotted from general to specific: `date.iso8601`, `email.basic`. It ends
    /// up in the output and in the index, so renaming one is a data migration.
    fn id(&self) -> &'static str;

    fn entity_type(&self) -> EntityType;

    /// The guard-free candidate pattern for the prefilter.
    fn candidate_pattern(&self) -> &'static str;

    /// Decides whether the candidate is real and what it means. `None` rejects it.
    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict>;
}

/// Every rule the scanner knows about.
pub fn all() -> Vec<Box<dyn Rule>> {
    vec![Box::new(email::EmailRule), Box::new(date_iso::Iso8601Rule)]
}
