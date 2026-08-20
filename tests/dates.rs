//! ISO 8601 / RFC 3339 rule: calendar validity, offset canonicalisation, and the guards that keep
//! identifiers out of the date facet.

mod support;

use regex_entity_scanner::model::{DatePrecision, EntityType, Flag, Value};

fn date_value(text: &str) -> (String, DatePrecision, bool) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert_eq!(entities.len(), 1, "{text:?} produced {entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::Date);
    match &entities[0].value {
        Value::Date {
            rfc3339,
            precision,
            tz_known,
        } => (rfc3339.clone(), *precision, *tz_known),
        other => panic!("expected a date value, got {other:?}"),
    }
}

#[test]
fn a_bare_date_carries_day_precision_and_no_zone() {
    let (rfc3339, precision, tz_known) = date_value("filed 2021-03-04 in Bucharest");
    assert_eq!(rfc3339, "2021-03-04");
    assert_eq!(precision, DatePrecision::Day);
    assert!(!tz_known);
}

/// Two spellings of the same instant have to land on the same value, or a range query over the
/// index silently misses half of its own corpus.
#[test]
fn offsets_canonicalise_to_one_spelling() {
    for text in ["2021-03-04T09:12:00+0200", "2021-03-04T09:12:00+02:00"] {
        let (rfc3339, precision, tz_known) = date_value(text);
        assert_eq!(rfc3339, "2021-03-04T09:12:00+02:00");
        assert_eq!(precision, DatePrecision::Second);
        assert!(tz_known);
    }
    let (rfc3339, _, tz_known) = date_value("2021-03-04t09:12:00z");
    assert_eq!(rfc3339, "2021-03-04T09:12:00Z");
    assert!(tz_known);
}

#[test]
fn a_zoneless_timestamp_says_so() {
    let scanner = support::scanner();
    let entities = scanner.scan("2021-03-04 09:12", 0);
    assert_eq!(entities.len(), 1);
    assert!(entities[0].flags.contains(&Flag::NoTimezone));
}

/// The pattern cannot know that February has 28 days. The validator can, which is why the check
/// belongs there and not in a cleverer pattern.
#[test]
fn rejects_dates_the_calendar_does_not_have() {
    let scanner = support::scanner();
    for text in [
        "2021-02-30",
        "2021-13-01",
        "2021-00-10",
        "2021-03-04T25:00:00Z",
    ] {
        let entities = scanner.scan(text, 0);
        assert!(
            entities.iter().all(|e| e.text != text),
            "{text:?} produced {entities:?}"
        );
    }
}

/// A date wedged inside a longer digit run is part of that run, not a date. This is the stripped
/// lookaround being re-imposed in the validator.
#[test]
fn rejects_dates_embedded_in_longer_numbers() {
    let scanner = support::scanner();
    for text in ["ref 992021-03-0412", "id=2021-03-041"] {
        let entities = scanner.scan(text, 0);
        assert!(entities.is_empty(), "{text:?} produced {entities:?}");
    }
}

#[test]
fn rejects_years_outside_the_plausible_window() {
    let scanner = support::scanner();
    let entities = scanner.scan("version 0001-02-03 of the schema", 0);
    assert!(entities.is_empty(), "{entities:?}");
}

/// Within one type the longer, more completely parsed match wins: a full timestamp is not also a
/// bare date.
#[test]
fn a_timestamp_is_emitted_once() {
    let scanner = support::scanner();
    let entities = scanner.scan("2021-03-04T09:12:00Z", 0);
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].text, "2021-03-04T09:12:00Z");
}

/// A rejected candidate is not the end of the story. The prefilter takes the longest match, so an
/// impossible clock swallows the perfectly good date in front of it; the scan loop looks inside the
/// rejection and the day survives.
#[test]
fn an_impossible_clock_leaves_the_date_inside_it() {
    let scanner = support::scanner();
    let entities = scanner.scan("logged 2021-03-04T25:00:00Z then 2021-03-05", 0);

    let texts: Vec<&str> = entities.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, ["2021-03-04", "2021-03-05"], "{entities:?}");
}

/// The recovery is bounded, and it does not turn a guard into a suggestion: a date wedged in a
/// longer digit run stays rejected however many times the window is shrunk, because the validator
/// still sees the whole fragment and the real neighbouring bytes.
#[test]
fn shrinking_a_candidate_does_not_defeat_the_adjacent_digit_guard() {
    let scanner = support::scanner();
    let entities = scanner.scan("ref 992021-03-0412 recorded", 0);
    assert!(entities.is_empty(), "{entities:?}");
}

/// The weekday is the strong half of a mail header date: a header whose day of the week disagrees
/// with its calendar date has been mangled or invented.
#[test]
fn a_mail_header_date_needs_a_consistent_weekday() {
    let (rfc3339, precision, tz_known) =
        date_value("Date: Thu, 04 Mar 2021 09:12:00 +0200\r\nFrom: ops");
    assert_eq!(rfc3339, "2021-03-04T09:12:00+02:00");
    assert_eq!(precision, DatePrecision::Second);
    assert!(tz_known);

    let scanner = support::scanner();
    let entities = scanner.scan("Date: Tue, 04 Mar 2021 09:12:00 +0200", 0);
    assert!(entities.is_empty(), "{entities:?}");
}

/// The obsolete zone abbreviations are still emitted by real mail software, and two spellings of
/// the same instant have to land on one value.
#[test]
fn obsolete_zone_abbreviations_normalise() {
    let (rfc3339, ..) = date_value("Sent Thu, 04 Mar 2021 09:12:00 GMT by the gateway");
    assert_eq!(rfc3339, "2021-03-04T09:12:00Z");
    let (rfc3339, ..) = date_value("Sent Thu, 04 Mar 2021 09:12:00 PST by the gateway");
    assert_eq!(rfc3339, "2021-03-04T09:12:00-08:00");
}

/// The brackets make the log timestamp self-identifying, and they are not part of the date the
/// reader sees highlighted.
#[test]
fn a_log_timestamp_reports_the_span_inside_its_brackets() {
    let scanner = support::scanner();
    let entities = scanner.scan(
        r#"10.0.0.4 - - [04/Mar/2021:09:12:00 +0200] "GET / HTTP/1.1" 200"#,
        0,
    );
    // The line also carries the client address, which is a second entity rather than a rival
    // reading of the same span.
    assert_eq!(entities.len(), 2, "{entities:?}");
    let date = entities
        .iter()
        .find(|entity| entity.rule_id == "date.clf")
        .expect("the log timestamp");
    assert_eq!(date.text, "04/Mar/2021:09:12:00 +0200");
    match &date.value {
        Value::Date { rfc3339, .. } => assert_eq!(rfc3339, "2021-03-04T09:12:00+02:00"),
        other => panic!("expected a date value, got {other:?}"),
    }
}

/// Week 53 exists only in the years that have one, which is the arithmetic that makes an ISO week
/// date a checked format rather than a shape.
#[test]
fn iso_week_dates_resolve_to_a_calendar_day() {
    let (rfc3339, precision, tz_known) = date_value("run 2021-W09-4 of the batch");
    assert_eq!(rfc3339, "2021-03-04");
    assert_eq!(precision, DatePrecision::Day);
    assert!(!tz_known);

    let (rfc3339, ..) = date_value("run 2021W094 of the batch");
    assert_eq!(rfc3339, "2021-03-04");

    let scanner = support::scanner();
    for text in ["batch 2021-W53-1", "batch 2021-W09-8", "batch 2021-W00-1"] {
        let entities = scanner.scan(text, 0);
        assert!(entities.is_empty(), "{text:?} produced {entities:?}");
    }
}

/// A date rule emits nothing unless the match fixes year, month and day. A week without a weekday
/// names a week, and a syslog line names no year at all.
#[test]
fn formats_that_do_not_fix_a_whole_day_are_not_matched() {
    let scanner = support::scanner();
    for text in ["reported in 2021-W09", "Mar  4 09:12:00 host sshd[1]: ok"] {
        let entities = scanner.scan(text, 0);
        assert!(entities.is_empty(), "{text:?} produced {entities:?}");
    }
}
