//! ISO 8601 / RFC 3339 dates and timestamps.
//!
//! Machine formats are the highest-precision date material in any corpus: they are unambiguous
//! about field order, they are common in logs, headers and exports, and they need no locale prior.
//! Human-written dates need a month-name lexicon and an order-disambiguation policy, and are a
//! separate rule.

use crate::model::{DatePrecision, EntityType, Flag, Value};
use crate::rules::{Candidate, Rule, Verdict};

pub struct Iso8601Rule;

/// Date, then optionally a time, then optionally an offset. Field ranges are checked in the
/// validator rather than in the pattern, because `jiff` knows about leap years and a regex does not.
const PATTERN: &str =
    r"\d{4}-\d{2}-\d{2}(?:[Tt ]\d{2}:\d{2}(?::\d{2}(?:\.\d{1,9})?)?(?:Z|z|[+\-]\d{2}:?\d{2})?)?";

/// Years outside this window are almost always a version string, an identifier fragment or an
/// off-by-a-digit typo rather than a date somebody meant to write.
const PLAUSIBLE_YEARS: std::ops::RangeInclusive<i16> = 1400..=2200;

impl Rule for Iso8601Rule {
    fn id(&self) -> &'static str {
        "date.iso8601"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Date
    }

    fn candidate_pattern(&self) -> &'static str {
        PATTERN
    }

    fn validate(&self, candidate: &Candidate<'_>) -> Option<Verdict> {
        // Stripped `(?<!\d)` and `(?!\d)` guards: a date wedged between digits is part of a longer
        // number, which is how identifiers and hashes turn into a corrupted date facet.
        if candidate.byte_before().is_some_and(|b| b.is_ascii_digit()) {
            return None;
        }
        if candidate.byte_after().is_some_and(|b| b.is_ascii_digit()) {
            return None;
        }

        let text = candidate.text();
        let (date_part, rest) = text.split_at(10);

        let year: i16 = date_part[0..4].parse().ok()?;
        let month: i8 = date_part[5..7].parse().ok()?;
        let day: i8 = date_part[8..10].parse().ok()?;

        if !PLAUSIBLE_YEARS.contains(&year) {
            return None;
        }
        // Rejects 2021-02-30 and 2021-13-01 for the right reason rather than by pattern.
        let date = jiff::civil::Date::new(year, month, day).ok()?;

        if rest.is_empty() {
            return Some(Verdict {
                start: candidate.start,
                end: candidate.end,
                value: Value::Date {
                    rfc3339: date.to_string(),
                    precision: DatePrecision::Day,
                    tz_known: false,
                },
                confidence: 0.99,
                flags: vec![Flag::NoTimezone],
            });
        }

        let time_part = &rest[1..];
        let (clock, offset) = split_offset(time_part);

        let hour: i8 = clock.get(0..2)?.parse().ok()?;
        let minute: i8 = clock.get(3..5)?.parse().ok()?;
        let (second, precision) = match clock.get(6..8) {
            Some(s) => (s.parse().ok()?, DatePrecision::Second),
            None => (0i8, DatePrecision::Minute),
        };
        // Constructing it is the range check; `jiff` rejects 25:00 and 12:61.
        jiff::civil::DateTime::new(year, month, day, hour, minute, second, 0).ok()?;

        let (suffix, tz_known, flags) = match offset {
            Some(offset) => (normalise_offset(offset)?, true, Vec::new()),
            None => (String::new(), false, vec![Flag::NoTimezone]),
        };

        let rfc3339 = match precision {
            DatePrecision::Second => format!("{date}T{hour:02}:{minute:02}:{second:02}{suffix}"),
            _ => format!("{date}T{hour:02}:{minute:02}{suffix}"),
        };

        Some(Verdict {
            start: candidate.start,
            end: candidate.end,
            value: Value::Date {
                rfc3339,
                precision,
                tz_known,
            },
            confidence: 0.99,
            flags,
        })
    }
}

/// Splits the clock from a trailing `Z` or `±HH:MM`, which the pattern allows to be attached
/// directly to the seconds field.
fn split_offset(time_part: &str) -> (&str, Option<&str>) {
    if let Some(clock) = time_part.strip_suffix(['Z', 'z']) {
        return (clock, Some("Z"));
    }
    // Search from the right: the sign cannot appear inside a clock, so the last one is the offset.
    if let Some(index) = time_part.rfind(['+', '-']) {
        return (&time_part[..index], Some(&time_part[index..]));
    }
    (time_part, None)
}

/// Canonicalises `Z`, `+0200` and `+02:00` to the one RFC 3339 spelling, so two identical instants
/// written differently land on the same value in the index.
fn normalise_offset(offset: &str) -> Option<String> {
    if offset.eq_ignore_ascii_case("z") {
        return Some("Z".to_string());
    }
    let sign = offset.chars().next()?;
    let digits: String = offset[1..].chars().filter(char::is_ascii_digit).collect();
    if digits.len() != 4 {
        return None;
    }
    let hours: u8 = digits[0..2].parse().ok()?;
    let minutes: u8 = digits[2..4].parse().ok()?;
    if hours > 18 || minutes > 59 {
        return None;
    }
    Some(format!("{sign}{hours:02}:{minutes:02}"))
}
