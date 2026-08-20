//! Sums of money: an amount plus the currency it is denominated in.
//!
//! Two properties decide the shape of this module.
//!
//! **The value is an integer.** A sum of money is a scaled integer, an ISO 4217 code and the
//! exponent that code divides into — never a float, at any point, including intermediates. Binary
//! floating point cannot represent a tenth, so a pipeline that touches one has already lost the
//! cent it was asked to preserve, and it loses it silently.
//!
//! **The separators are a convention, not a grammar.** `1.234` is a thousand in Bucharest and
//! one-and-a-bit in London, and no amount of looking at the digits settles it. What the digits do
//! settle is which readings are impossible: a group of exactly three digits after the only
//! separator is the case where both readings survive, and that is what the separator-inferred flag
//! is for. Every other shape is decided by arithmetic — a group that is not three digits long
//! cannot be a thousands group, and a separator followed by one or two digits cannot be anything
//! but a decimal mark.

use crate::model::{EntityType, Flag, Value};
use crate::rules::context::DEFAULT_CUE_WINDOW;
use crate::rules::{Candidate, Rule, Verdict};

/// Codes that are also ordinary English words in the register where they appear in capitals: ALL
/// items, TOP 40, the CUP final. Membership of ISO 4217 is not enough for these, because the
/// sentences they turn up in are far more common than the currencies are.
const WORD_LIKE_CODES: &[&str] = &[
    "ALL", "BOB", "COP", "CUP", "GEL", "MAD", "MOP", "PEN", "SOS", "TOP", "TRY",
];

/// What a sum of money is being talked about when the code is one of the word-like ones.
const MONEY_CUES: &[&str] = &[
    "price", "cost", "costs", "amount", "total", "paid", "pay", "invoice", "fee", "fees", "salary",
    "budget", "balance", "sum", "worth", "value", "revenue", "turnover", "currency",
];

// ---------------------------------------------------------------------------------------------
// ISO 4217 code
// ---------------------------------------------------------------------------------------------

pub struct IsoCodeRule;

/// `EUR 1.234,56` or `1,234.56 USD`. The code may lead or trail, because both are written and
/// neither is more correct. The amount half — digits optionally grouped by spaces, dots or commas
/// — is deliberately looser than any real number format: which separator means what is the
/// validator's problem, and the shapes that mean nothing at all are rejected there.
const ISO_CODE_PATTERN: &str = concat!(
    r"[A-Z]{3}[\x20\x{a0}]?",
    r"[0-9]+(?:[\x20\x{a0}\x{202f}][0-9]{3})*(?:[.,][0-9]+)*",
    r"|",
    r"[0-9]+(?:[\x20\x{a0}\x{202f}][0-9]{3})*(?:[.,][0-9]+)*",
    r"[\x20\x{a0}]?[A-Z]{3}"
);

impl Rule for IsoCodeRule {
    fn id(&self) -> &'static str {
        "money.iso_code"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Money
    }

    fn candidate_pattern(&self) -> &'static str {
        ISO_CODE_PATTERN
    }

    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict> {
        if candidate.byte_before().is_some_and(is_word_byte)
            || candidate.byte_after().is_some_and(is_word_byte)
        {
            return None;
        }

        let text = candidate.text();
        let leading = text.as_bytes().first()?.is_ascii_uppercase();
        let (code, amount) = if leading {
            (&text[..3], text[3..].trim_start())
        } else {
            (&text[text.len() - 3..], text[..text.len() - 3].trim_end())
        };

        let currency = candidate.data.currency(code)?;
        if WORD_LIKE_CODES.contains(&code) && !candidate.has_cue(MONEY_CUES, DEFAULT_CUE_WINDOW) {
            return None;
        }

        let (amount_minor, inferred) = to_minor_units(amount, currency.exponent)?;

        Some(Verdict {
            start: candidate.start,
            end: candidate.end,
            value: Value::Money {
                currency: code.to_string(),
                amount_minor,
                exponent: currency.exponent,
            },
            // There is no check digit here to earn a higher number: what the validator has is the
            // amount's structure and the code's membership of the ISO 4217 list in current use,
            // which is the ladder's own description of 0.95.
            confidence: 0.95,
            flags: if inferred {
                vec![Flag::SeparatorInferred]
            } else {
                Vec::new()
            },
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Currency symbol
// ---------------------------------------------------------------------------------------------

pub struct SymbolRule;

/// A Unicode currency sign, optionally carrying the one or two capitals that distinguish the
/// dollars and yuan from each other — `A$`, `CN¥`, `R$`. `\p{Sc}` is the property the signs are
/// defined by, which is why the pattern does not enumerate them.
const SYMBOL_PATTERN: &str = concat!(
    r"[A-Z]{0,2}\p{Sc}[\x20\x{a0}]?",
    r"[0-9]+(?:[\x20\x{a0}\x{202f}][0-9]{3})*(?:[.,][0-9]+)*",
    r"|",
    r"[0-9]+(?:[\x20\x{a0}\x{202f}][0-9]{3})*(?:[.,][0-9]+)*",
    r"[\x20\x{a0}]?[A-Z]{0,2}\p{Sc}"
);

impl Rule for SymbolRule {
    fn id(&self) -> &'static str {
        "money.symbol"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Money
    }

    fn candidate_pattern(&self) -> &'static str {
        SYMBOL_PATTERN
    }

    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict> {
        if candidate.byte_before().is_some_and(is_word_byte)
            || candidate.byte_after().is_some_and(is_word_byte)
        {
            return None;
        }

        let text = candidate.text();
        let leading = !text.starts_with(|c: char| c.is_ascii_digit());
        let (symbol, amount) = if leading {
            let split = text.find(|c: char| c.is_ascii_digit())?;
            (text[..split].trim_end(), text[split..].trim_start())
        } else {
            let split = text.rfind(|c: char| c.is_ascii_digit())? + 1;
            (text[split..].trim_start(), text[..split].trim_end())
        };

        // A capital or two in front of the sign is part of the symbol only when the pair is one
        // somebody actually writes. Otherwise they belong to the word before it, and the span
        // narrows to the sign — `US$40` and `paid US $40` both mean forty dollars, and neither
        // means the letters are currency.
        let (symbol, start) = match candidate.data.currency_symbol(symbol) {
            Some(_) => (symbol, candidate.start),
            None => {
                let letters = symbol.len()
                    - symbol
                        .trim_start_matches(|c: char| c.is_ascii_uppercase())
                        .len();
                if letters == 0 || !leading {
                    return None;
                }
                let before = candidate.fragment.as_bytes()[..candidate.start].last();
                if before.is_some_and(|byte| byte.is_ascii_alphanumeric()) {
                    return None;
                }
                (&symbol[letters..], candidate.start + letters)
            }
        };

        let codes = candidate.data.currency_symbol(symbol)?;
        let code = codes.first()?;
        let currency = candidate.data.currency(code)?;
        let (amount_minor, inferred) = to_minor_units(amount, currency.exponent)?;

        let mut flags = Vec::new();
        if codes.len() > 1 {
            flags.push(Flag::AmbiguousCurrency);
        }
        if inferred {
            flags.push(Flag::SeparatorInferred);
        }

        Some(Verdict {
            start,
            end: candidate.end,
            value: Value::Money {
                currency: code.to_string(),
                amount_minor,
                exponent: currency.exponent,
            },
            // A symbol shared by twenty-nine currencies names the most widely used of them and says
            // so with a flag; it has not established which one this is.
            confidence: if codes.len() > 1 { 0.80 } else { 0.95 },
            flags,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Separator inference and scaling
// ---------------------------------------------------------------------------------------------

/// Whether a byte would make the span part of a longer token. Not `is_ascii_alphanumeric`, because
/// a separator on either side means the digits continue and the amount is not the whole number.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'.' || byte == b',' || byte == b'-'
}

/// The amount as an integer number of minor units, and whether the separators had to be guessed.
///
/// Ported from `vendored/reference/price-parser/`, whose rule is the last separator's group: one or
/// two digits, or four and more, can only be a fraction; exactly three digits is a thousands group
/// unless something else in the number says otherwise. Everything is `u128` arithmetic on digit
/// strings, so no intermediate is ever a float.
fn to_minor_units(amount: &str, exponent: u8) -> Option<(String, bool)> {
    // Space grouping carries no ambiguity at all, so it is settled before anything else looks at
    // the number.
    let amount: String = amount
        .chars()
        .filter(|c| !matches!(c, ' ' | '\u{a0}' | '\u{202f}'))
        .collect();
    if amount.is_empty() || !amount.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }

    let separators: Vec<usize> = amount
        .char_indices()
        .filter(|(_, c)| *c == '.' || *c == ',')
        .map(|(index, _)| index)
        .collect();

    let mut inferred = false;
    let decimal = match separators.last() {
        None => None,
        Some(&last) => {
            let tail = amount.len() - last - 1;
            // Two different separator characters in one number explain each other: `1.234,567`
            // cannot be read the other way round, whatever the length of the last group.
            let mixed = separators.len() > 1
                && amount.as_bytes()[separators[separators.len() - 2]] != amount.as_bytes()[last];
            if tail == 3 && !mixed {
                // Grouping, and where it is the only separator the reading was a choice: `1,234`
                // is a thousand or one-and-a-bit depending on who wrote it.
                inferred = separators.len() == 1;
                None
            } else {
                Some(last)
            }
        }
    };

    // Every separator that is not the decimal mark is a thousands group, and a thousands group is
    // exactly three digits. This is what rejects `1.2.3` and every other dotted run that is a
    // version, a chapter number or an address rather than a number.
    let groups: Vec<usize> = separators
        .iter()
        .copied()
        .filter(|index| Some(*index) != decimal)
        .collect();
    if !groups.is_empty() {
        let first = groups[0];
        if first == 0 || first > 3 {
            return None;
        }
        let mut previous = first;
        for &index in &groups[1..] {
            if index - previous != 4 {
                return None;
            }
            previous = index;
        }
        let after_last = decimal.unwrap_or(amount.len());
        if after_last - previous != 4 {
            return None;
        }
        // Mixed separators are self-explaining: `1.234,56` cannot be read the other way round.
        let group_char = amount.as_bytes()[first];
        if groups
            .iter()
            .any(|index| amount.as_bytes()[*index] != group_char)
        {
            return None;
        }
        if let Some(decimal) = decimal {
            if amount.as_bytes()[decimal] == group_char {
                return None;
            }
        }
    }

    let (units, fraction) = match decimal {
        Some(index) => (&amount[..index], &amount[index + 1..]),
        None => (&amount[..], ""),
    };
    let units: String = units.chars().filter(char::is_ascii_digit).collect();
    if units.is_empty() || units.len() > 24 {
        return None;
    }

    // A currency with two minor units is not written with four decimal places, and something that
    // is written that way is a rate, a measurement or a version — not a sum of money.
    let exponent = usize::from(exponent);
    if fraction.len() > exponent {
        return None;
    }
    let minor = format!("{units}{fraction}{}", "0".repeat(exponent - fraction.len()));
    let minor: String = minor.trim_start_matches('0').to_string();
    Some((
        if minor.is_empty() {
            "0".to_string()
        } else {
            minor
        },
        inferred,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separator_inference_follows_the_upstream_rule() {
        // Two decimal places are a fraction, three are a thousands group, and the mixed forms are
        // decided by the pair rather than by either separator alone.
        assert_eq!(
            to_minor_units("1.234,56", 2),
            Some(("123456".into(), false))
        );
        assert_eq!(
            to_minor_units("1,234.56", 2),
            Some(("123456".into(), false))
        );
        assert_eq!(to_minor_units("12,34", 2), Some(("1234".into(), false)));
        assert_eq!(to_minor_units("1200", 2), Some(("120000".into(), false)));
        assert_eq!(
            to_minor_units("1 234,56", 2),
            Some(("123456".into(), false))
        );
        // The one genuinely ambiguous shape: read as a thousand, and said to have been read.
        assert_eq!(to_minor_units("1,234", 2), Some(("123400".into(), true)));
        assert_eq!(to_minor_units("1.234", 2), Some(("123400".into(), true)));
        // The exponent is the currency's, not two.
        assert_eq!(
            to_minor_units("1.234,567", 3),
            Some(("1234567".into(), false))
        );
        assert_eq!(to_minor_units("1200", 0), Some(("1200".into(), false)));
    }

    #[test]
    fn shapes_that_are_not_numbers_are_rejected() {
        // A version string, a chapter number and a date all reach this function through some
        // pattern or other, and none of them has three-digit thousands groups.
        assert_eq!(to_minor_units("1.2.3", 2), None);
        assert_eq!(to_minor_units("1.2345.67", 2), None);
        assert_eq!(to_minor_units("12,3456", 2), None);
        assert_eq!(to_minor_units("1.234.56", 2), None);
        assert_eq!(to_minor_units("", 2), None);
        assert_eq!(to_minor_units("1.5", 0), None);
    }
}
