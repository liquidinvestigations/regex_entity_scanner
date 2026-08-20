//! National identity numbers, detection only.
//!
//! The fixtures are the documented examples from `stdnum/it/codicefiscale.py`, `stdnum/es/dni.py`,
//! `stdnum/es/nie.py`, `stdnum/mx/curp.py` and `stdnum/in_/pan.py` — each module's valid number
//! and the counterpart that differs only in its check character. The subset covers what each
//! scheme claims: the codice fiscale's two-table check character, the Spanish modulo-23 letter in
//! both its resident and its foreign-national spelling, the CURP's weighted check digit and state
//! code, and the PAN's holder-type letter and the cue word it needs because its check character is
//! not a published algorithm.
//!
//! Nothing here asserts a date of birth or a sex, because nothing produces one.

mod support;

use regex_entity_scanner::model::{EntityType, Flag, Value};

fn scheme_and_country(text: &str) -> (String, String) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert_eq!(entities.len(), 1, "{text:?} produced {entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::NationalId);
    match &entities[0].value {
        Value::Identifier {
            scheme, country, ..
        } => (
            scheme.clone(),
            country.clone().unwrap_or_else(|| "?".to_string()),
        ),
        other => panic!("expected an identifier value, got {other:?}"),
    }
}

fn rejects(text: &str) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert!(entities.is_empty(), "{text:?} produced {entities:?}");
}

#[test]
fn accepts_a_documented_codice_fiscale() {
    assert_eq!(
        scheme_and_country("taxpayer RCCMNL83S18D969H filed late"),
        ("it_codice_fiscale".to_string(), "IT".to_string())
    );
}

#[test]
fn rejects_a_codice_fiscale_whose_check_character_disagrees() {
    rejects("taxpayer RCCMNL83S18D969A filed late");
}

#[test]
fn accepts_spanish_numbers_in_both_spellings() {
    assert_eq!(
        scheme_and_country("resident 54362315K on the register"),
        ("es_nif_nie".to_string(), "ES".to_string())
    );
    assert_eq!(
        scheme_and_country("foreign national X2482300W on the register"),
        ("es_nif_nie".to_string(), "ES".to_string())
    );
    assert_eq!(
        scheme_and_country("holder M1234567L on the register"),
        ("es_nif_nie".to_string(), "ES".to_string())
    );
}

#[test]
fn rejects_spanish_numbers_whose_check_letter_disagrees() {
    rejects("resident 54362315Z on the register");
    rejects("foreign national X2482300A on the register");
}

#[test]
fn accepts_a_documented_curp() {
    assert_eq!(
        scheme_and_country("registered as BOXW310820HNERXN09"),
        ("mx_curp".to_string(), "MX".to_string())
    );
}

#[test]
fn rejects_curps_with_a_bad_check_digit_or_an_unknown_state() {
    rejects("registered as BOXW310820HNERXN08");
    // ZZ is not a state the registry uses.
    rejects("registered as BOXW310820HZZRXN09");
}

/// The check character's algorithm has never been published, so structure plus a word nearby is
/// the whole precision story — and the flag and the confidence say exactly that.
#[test]
fn a_pan_needs_a_word_beside_it_and_reports_no_checksum() {
    let scanner = support::scanner();
    let entities = scanner.scan("PAN ACUPA7085R on the return", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert!(entities[0].flags.contains(&Flag::NoChecksum));
    assert!(entities[0].confidence < 0.95, "{entities:?}");

    rejects("code ACUPA7085R in the export");
}

#[test]
fn rejects_pans_with_an_impossible_holder_type_or_serial() {
    // X is not a holder type the department assigns.
    rejects("PAN ABMXA3211G on the return");
    // The serial runs from 0001.
    rejects("PAN ACUPA0000R on the return");
}
