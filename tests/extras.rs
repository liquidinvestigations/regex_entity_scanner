//! The self-identifying extras: CVE ids, DOIs, ORCIDs, message ids.
//!
//! Each of these earns its precision from a literal marker rather than from arithmetic, so the
//! subset here is chosen for the two ways that can fail: a marker with an implausible payload
//! behind it, and a shape that carries the marker but means something else — an angle-bracketed
//! address in a `From` line rather than a message id.

mod support;

use regex_entity_scanner::model::{EntityType, Value};

fn only_match(text: &str) -> (String, String) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert_eq!(entities.len(), 1, "{text:?} produced {entities:?}");
    match &entities[0].value {
        Value::Identifier {
            scheme, compact, ..
        } => (scheme.clone(), compact.clone()),
        other => panic!("expected an identifier value, got {other:?}"),
    }
}

fn rejects(text: &str) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert!(entities.is_empty(), "{text:?} produced {entities:?}");
}

#[test]
fn accepts_cve_identifiers_and_reports_their_parts() {
    let scanner = support::scanner();
    let entities = scanner.scan("patched CVE-2021-44228 on Friday", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::Vulnerability);
    match &entities[0].value {
        Value::Identifier { compact, parts, .. } => {
            assert_eq!(compact, "CVE-2021-44228");
            assert_eq!(parts.get("year").map(String::as_str), Some("2021"));
            assert_eq!(parts.get("sequence").map(String::as_str), Some("44228"));
        }
        other => panic!("expected an identifier value, got {other:?}"),
    }
    assert_eq!(only_match("see CVE-2014-0160").1, "CVE-2014-0160");
}

#[test]
fn rejects_cve_shaped_runs_outside_the_scheme() {
    // The scheme starts in 1999.
    rejects("reference CVE-1066-0160 in the file");
    // A longer hyphenated run is not a CVE with something after it.
    rejects("token CVE-2021-44228-B installed");
}

#[test]
fn accepts_dois_and_trims_the_sentence_punctuation() {
    assert_eq!(
        only_match("published as 10.1000/182 last year"),
        ("doi".to_string(), "10.1000/182".to_string())
    );
    let scanner = support::scanner();
    let entities = scanner.scan("see doi 10.1145/3292500.3330701.", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].text, "10.1145/3292500.3330701");
}

#[test]
fn rejects_doi_shaped_runs_with_no_suffix() {
    rejects("version 10.1000/ was withdrawn");
    // A directory code that is part of a longer number is not a DOI.
    rejects("figure 110.1000/182 of the table");
}

#[test]
fn accepts_orcids_including_the_x_check_character() {
    for orcid in [
        "0000-0002-1825-0097",
        "0000-0001-5109-3700",
        "0000-0002-1694-233X",
    ] {
        let text = format!("author {orcid} of the paper");
        assert_eq!(only_match(&text), ("orcid".to_string(), orcid.to_string()));
    }
}

#[test]
fn rejects_orcids_whose_check_character_disagrees() {
    rejects("author 0000-0002-1825-0098 of the paper");
    rejects("author 0000-0001-5109-3701 of the paper");
}

#[test]
fn accepts_a_message_id_behind_its_header_name() {
    let scanner = support::scanner();
    let entities = scanner.scan("Message-ID: <20210304091200.ABC123@mail.example.com>", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::MessageId);
    assert_eq!(entities[0].text, "<20210304091200.ABC123@mail.example.com>");
    match &entities[0].value {
        Value::Identifier { compact, parts, .. } => {
            assert_eq!(compact, "20210304091200.ABC123@mail.example.com");
            assert_eq!(
                parts.get("domain").map(String::as_str),
                Some("mail.example.com")
            );
        }
        other => panic!("expected an identifier value, got {other:?}"),
    }
}

/// An address in a `From` line has exactly the shape of a message id, and reading it as one both
/// invents an entity and hides the address behind it.
#[test]
fn an_angle_bracketed_address_without_a_header_name_stays_an_address() {
    let scanner = support::scanner();
    let entities = scanner.scan("From: Anna Popescu <anna.popescu@example.co.uk>", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::Email);
}

#[test]
fn rejects_a_message_id_whose_domain_has_no_registered_suffix() {
    rejects("Message-ID: <20210304091200.ABC123@mail.example.zzz>");
}

fn geo_point(text: &str) -> (f64, f64) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert_eq!(entities.len(), 1, "{text:?} produced {entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::Coordinates);
    match &entities[0].value {
        Value::GeoPoint {
            latitude,
            longitude,
            datum,
        } => {
            assert_eq!(datum, "WGS84");
            (*latitude, *longitude)
        }
        other => panic!("expected a geo point value, got {other:?}"),
    }
}

#[test]
fn accepts_decimal_degrees_with_enough_precision() {
    assert_eq!(
        geo_point("meet at 48.8583, 2.2945 tomorrow"),
        (48.8583, 2.2945)
    );
    assert_eq!(geo_point("sighted -33.8568,151.2153"), (-33.8568, 151.2153));
}

/// Two numbers with three decimal places are a pair of measurements as often as a place, and a
/// latitude past the pole is neither.
#[test]
fn rejects_number_pairs_that_are_not_coordinates() {
    rejects("readings of 12.345, 67.890 were logged");
    rejects("point 91.1234, 2.2945 is off the earth");
}

#[test]
fn accepts_the_sexagesimal_form_and_converts_it() {
    let (latitude, longitude) = geo_point("marker at 40°26'46\"N 79°58'56\"W");
    assert!((latitude - 40.446_111).abs() < 1e-5, "{latitude}");
    assert!((longitude + 79.982_222).abs() < 1e-5, "{longitude}");
}

#[test]
fn rejects_sexagesimal_readings_that_have_run_over() {
    rejects("marker at 40°26'46\"N 79°78'56\"W");
}

/// The ten-character cell is about fourteen metres across, so the centre is what is reported.
#[test]
fn accepts_a_plus_code_and_reports_its_cell_centre() {
    let (latitude, longitude) = geo_point("the office is at 8FVC9G8F+6W");
    assert!((latitude - 47.365_5).abs() < 1e-3, "{latitude}");
    assert!((longitude - 8.524_8).abs() < 1e-3, "{longitude}");
}

#[test]
fn rejects_plus_codes_outside_the_alphabet_or_the_range() {
    // A latitude character past the ninth puts the cell north of the pole.
    rejects("code XFVC9G8F+6W is not a place");
    // The alphabet has no vowels, so this is a word with a plus after it.
    rejects("code 8FVCAG8F+6W is not a place");
}
