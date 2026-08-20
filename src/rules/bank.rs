//! Bank account and bank institution identifiers: IBAN and BIC.

use std::collections::BTreeMap;

use crate::model::{EntityType, Flag, Value};
use crate::rules::checksum::iso7064;
use crate::rules::context::DEFAULT_CUE_WINDOW;
use crate::rules::{Candidate, Rule, Verdict};

// ---------------------------------------------------------------------------------------------
// IBAN
// ---------------------------------------------------------------------------------------------

pub struct IbanRule;

/// Deliberately loose: two letters, two check digits and a run that may carry the spaces IBANs are
/// written in as often as not. Length, alphabet and arithmetic are all the validator's work,
/// because they are per country and a pattern that encoded them would be ninety alternations.
const IBAN_PATTERN: &str = r"[A-Z]{2}\d{2}[A-Z0-9 ]{10,34}";

impl Rule for IbanRule {
    fn id(&self) -> &'static str {
        "bank.iban"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::BankAccount
    }

    fn candidate_pattern(&self) -> &'static str {
        IBAN_PATTERN
    }

    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict> {
        // Stripped `(?<![A-Z0-9])`: an IBAN growing out of a longer token is part of that token.
        // There is no matching guard on the right, and that is deliberate — the registry length is
        // exact, so a candidate whose tail is a page number is rejected on length and the scan
        // loop then finds the IBAN inside it.
        if candidate
            .byte_before()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        {
            return None;
        }

        // The pattern's character class contains the space, so a match can end in one.
        let text = candidate.text().trim_end_matches(' ');
        let end = candidate.start + text.len();

        let compact: String = text.chars().filter(|c| *c != ' ').collect();
        let country_code = compact.get(0..2)?.to_string();
        let registry = candidate.data.iban_country(&country_code)?;

        if compact.len() != registry.length {
            return None;
        }
        if !matches_structure(compact.get(4..)?, &registry.structure) {
            return None;
        }
        // ISO 7064 mod-97-10 over the rearranged form: the country code and check digits move to
        // the end, which is what puts them inside the arithmetic.
        if !iso7064::mod_97_10(&format!("{}{}", &compact[4..], &compact[..4])) {
            return None;
        }

        let mut parts = BTreeMap::new();
        parts.insert("bban".to_string(), compact[4..].to_string());
        parts.insert("check_digits".to_string(), compact[2..4].to_string());
        if let Some([from, to]) = registry.bank {
            if let Some(bank) = compact.get(from..to) {
                parts.insert("bank_code".to_string(), bank.to_string());
            }
        }

        Some(Verdict {
            start: candidate.start,
            end,
            value: Value::Identifier {
                scheme: "iban".to_string(),
                compact,
                country: Some(country_code),
                parts,
            },
            confidence: 0.99,
            flags: Vec::new(),
        })
    }
}

/// Whether a BBAN satisfies a registry structure string — a run of `<length>!<class>` groups where
/// `n` is digits, `a` letters and `c` alphanumerics.
///
/// This is the half of IBAN validation that mod-97 does not do: the checksum accepts about one
/// malformed candidate in ninety-seven, and the positional alphabet is what closes that gap.
fn matches_structure(bban: &str, structure: &str) -> bool {
    let mut rest = bban.as_bytes();
    let mut spec = structure;

    while !spec.is_empty() {
        let Some(bang) = spec.find('!') else {
            return false;
        };
        let Ok(count) = spec[..bang].parse::<usize>() else {
            return false;
        };
        let Some(class) = spec.as_bytes().get(bang + 1).copied() else {
            return false;
        };
        spec = &spec[bang + 2..];

        if rest.len() < count {
            return false;
        }
        let (group, remainder) = rest.split_at(count);
        rest = remainder;

        let ok = group.iter().all(|byte| match class {
            b'n' => byte.is_ascii_digit(),
            b'a' => byte.is_ascii_uppercase(),
            b'c' => byte.is_ascii_alphanumeric(),
            _ => false,
        });
        if !ok {
            return false;
        }
    }

    rest.is_empty()
}

// ---------------------------------------------------------------------------------------------
// BIC
// ---------------------------------------------------------------------------------------------

pub struct BicRule;

/// Four letters of institution code, two of country, two of location, and an optional three-letter
/// branch. ISO 9362 defines no check digit, so everything precise about this rule is in the
/// validator.
const BIC_PATTERN: &str = r"[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}(?:[A-Z0-9]{3})?";

/// A BIC is eight or eleven uppercase characters with no checksum, which is a shape ordinary
/// English words in a heading also have — `DOCUMENT` and `CUSTOMER` both carry a valid ISO 3166-1
/// country code in positions five and six. The label beside the code is what separates a bank
/// identifier from a capitalised word, so it is required.
const BIC_CUES: &[&str] = &[
    "bic",
    "swift",
    "swiftbic",
    "iban",
    "bank",
    "beneficiary",
    "correspondent",
];

impl Rule for BicRule {
    fn id(&self) -> &'static str {
        "bank.bic"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::BankAccount
    }

    fn candidate_pattern(&self) -> &'static str {
        BIC_PATTERN
    }

    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict> {
        // The length is fixed at eight or eleven, so both neighbours are guards: a code cut out of
        // a longer token is not a code.
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
        if text.len() != 8 && text.len() != 11 {
            return None;
        }

        let country_code = text.get(4..6)?;
        // Membership in the country list is the structural half of the precision story: it is what
        // makes positions five and six mean something.
        if !candidate.data.is_country_code(country_code) {
            return None;
        }

        if !candidate.has_cue(BIC_CUES, DEFAULT_CUE_WINDOW) {
            return None;
        }

        let mut parts = BTreeMap::new();
        parts.insert("institution".to_string(), text[0..4].to_string());
        parts.insert("location".to_string(), text[6..8].to_string());
        if text.len() == 11 {
            parts.insert("branch".to_string(), text[8..11].to_string());
        }

        Some(Verdict {
            start: candidate.start,
            end: candidate.end,
            value: Value::Identifier {
                scheme: "bic".to_string(),
                compact: text.to_string(),
                country: Some(country_code.to_string()),
                parts,
            },
            confidence: 0.95,
            flags: vec![Flag::NoChecksum],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The structure strings from the registry entries for Romania, Germany and the United Kingdom,
    /// which between them exercise all three character classes.
    #[test]
    fn structures_are_checked_positionally() {
        assert!(matches_structure("AAAA1B31007593840000", "4!a16!c"));
        assert!(!matches_structure("1AAA1B31007593840000", "4!a16!c"));
        assert!(matches_structure("370400440532013000", "8!n10!n"));
        assert!(!matches_structure("37040044053201300A", "8!n10!n"));
        assert!(matches_structure("WEST12345698765432", "4!a6!n8!n"));
        // Too short and too long both fail, which is what makes the length check positional rather
        // than a prefix test.
        assert!(!matches_structure("WEST1234569876543", "4!a6!n8!n"));
        assert!(!matches_structure("WEST123456987654321", "4!a6!n8!n"));
    }
}
