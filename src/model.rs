//! The output contract.
//!
//! Offsets are byte offsets into the source document, not into the fragment that was scanned, and
//! not character indices. Fragments carry the offset of their first byte and the scanner adds it,
//! so a caller that splits a document into overlapping windows gets spans it can use directly
//! against the original bytes.

use serde::{Deserialize, Serialize};

/// The category a rule produces. The wire form is the string a consumer indexes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Date,
    Email,
}

impl EntityType {
    /// Decides which type wins when two accepted spans overlap. A higher number wins.
    ///
    /// Precedence is by structural strength, not by usefulness: a type whose acceptance depended on
    /// a checksum or a calendar check is far less likely to be the accidental reading of the two,
    /// so it outranks a type that only had to look right.
    pub fn precedence(self) -> u8 {
        match self {
            EntityType::Email => 20,
            EntityType::Date => 10,
        }
    }
}

/// A machine-readable caveat attached to an otherwise accepted match.
///
/// Flags are the alternative to guessing silently. An ambiguity that a document cannot resolve is
/// reported as an ambiguity; it is never resolved by coin flip and presented as certainty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flag {
    /// The day and month could be either way round and nothing in reach decides it.
    AmbiguousOrder,
    /// The value carries no time zone, so the instant is only defined up to the reader's guess.
    NoTimezone,
}

/// The canonical value a match normalises to, which is the entire point of the second stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Date {
        /// The instant or civil time in RFC 3339 form, truncated to `precision`.
        rfc3339: String,
        precision: DatePrecision,
        /// False when the offset was absent and had to be left unstated.
        tz_known: bool,
    },
    Email {
        /// Local part unchanged, domain lowercased — the RFC-correct normalisation.
        address: String,
        local: String,
        domain: String,
    },
}

/// How far down a date was actually specified. A date field indexed at the wrong precision reads
/// as spurious accuracy, so the precision travels with the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatePrecision {
    Day,
    Minute,
    Second,
}

/// One accepted match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    /// Byte offset of the first byte of the match in the source document.
    pub start: usize,
    /// Byte offset one past the last byte of the match.
    pub end: usize,
    /// The surface form exactly as it appears, for highlighting and for showing a user why.
    pub text: String,
    pub value: Value,
    pub confidence: f32,
    /// Which rule produced this. Without it, tracking down a bad facet over a real corpus means
    /// guessing, so it is part of the contract rather than a debug aid.
    pub rule_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<Flag>,
}
