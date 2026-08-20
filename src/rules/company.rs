//! Identifiers that name a legal entity rather than an account or an instrument.

use std::collections::BTreeMap;

use crate::model::{EntityType, Value};
use crate::rules::checksum::iso7064;
use crate::rules::{Candidate, Rule, Verdict};

pub struct LeiRule;

/// Twenty characters: four of issuing LOU, two usually zero, thirteen identifying the organisation
/// and two check digits. Fixed length, fixed alphabet, and the last two positions are digits — a
/// shape common enough to need the checksum, and rare enough that the checksum settles it.
const LEI_PATTERN: &str = r"[A-Z0-9]{18}\d{2}";

impl Rule for LeiRule {
    fn id(&self) -> &'static str {
        "company.lei"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::CompanyId
    }

    fn candidate_pattern(&self) -> &'static str {
        LEI_PATTERN
    }

    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict> {
        // The length is exactly twenty, so both neighbours are guards: twenty characters cut out
        // of a hash or a base32 blob are not an LEI.
        if candidate
            .byte_before()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        {
            return None;
        }
        if candidate
            .byte_after()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        {
            return None;
        }

        let text = candidate.text();
        // ISO 7064 mod-97-10 over the whole code, letters counting as their base-36 value.
        if !iso7064::mod_97_10(text) {
            return None;
        }

        let mut parts = BTreeMap::new();
        parts.insert("lou".to_string(), text[0..4].to_string());
        parts.insert("entity".to_string(), text[6..18].to_string());
        parts.insert("check_digits".to_string(), text[18..20].to_string());

        Some(Verdict {
            start: candidate.start,
            end: candidate.end,
            value: Value::Identifier {
                scheme: "lei".to_string(),
                compact: text.to_string(),
                country: None,
                parts,
            },
            confidence: 0.99,
            flags: Vec::new(),
        })
    }
}
