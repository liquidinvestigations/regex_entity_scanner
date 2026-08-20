//! Bank identifiers: IBAN and BIC.
//!
//! The fixtures are the documented examples from the `python-stdnum` modules this arithmetic is
//! ported from — `stdnum/iban.py` and the per-country `stdnum/*/iban.py` docstrings for the valid
//! numbers and their invalid-check-digit counterparts, `stdnum/bic.py` for the codes. The subset is
//! chosen to cover the three BBAN character classes, the shortest and longest registry lengths, and
//! the four ways an IBAN candidate can fail: unknown country, wrong length, wrong alphabet, wrong
//! checksum.

mod support;

use regex_entity_scanner::model::{EntityType, Flag, Value};

fn identifier(text: &str) -> (String, String, Option<String>) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert_eq!(entities.len(), 1, "{text:?} produced {entities:?}");
    match &entities[0].value {
        Value::Identifier {
            scheme,
            compact,
            country,
            ..
        } => (scheme.clone(), compact.clone(), country.clone()),
        other => panic!("expected an identifier value, got {other:?}"),
    }
}

fn rejects(text: &str) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert!(entities.is_empty(), "{text:?} produced {entities:?}");
}

#[test]
fn accepts_the_documented_ibans_in_both_spellings() {
    for (text, compact) in [
        (
            "IBAN GR16 0110 1050 0000 1054 7023 795",
            "GR1601101050000010547023795",
        ),
        ("IBAN BE31435411161155", "BE31435411161155"),
        ("IBAN NO93 8601 1117 947", "NO9386011117947"),
        ("IBAN ME25510000000006234133", "ME25510000000006234133"),
        ("IBAN GB82 WEST 1234 5698 7654 32", "GB82WEST12345698765432"),
        (
            "IBAN RO49 AAAA 1B31 0075 9384 0000",
            "RO49AAAA1B31007593840000",
        ),
    ] {
        let (scheme, found, country) = identifier(text);
        assert_eq!(scheme, "iban");
        assert_eq!(found, compact);
        assert_eq!(country.as_deref(), Some(&compact[0..2]));
    }
}

/// The country in the value comes from the number itself, and the parts are sliced where the
/// registry says the bank identifier sits.
#[test]
fn an_iban_carries_its_country_and_its_bank_code() {
    let scanner = support::scanner();
    let entities = scanner.scan("settle to IBAN GB82 WEST 1234 5698 7654 32 please", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    let entity = &entities[0];
    assert_eq!(entity.entity_type, EntityType::BankAccount);
    assert_eq!(entity.rule_id, "bank.iban");
    assert_eq!(entity.text, "GB82 WEST 1234 5698 7654 32");
    match &entity.value {
        Value::Identifier { parts, .. } => {
            assert_eq!(parts.get("bank_code").map(String::as_str), Some("WEST"));
            assert_eq!(parts.get("check_digits").map(String::as_str), Some("82"));
            assert_eq!(
                parts.get("bban").map(String::as_str),
                Some("WEST12345698765432")
            );
        }
        other => panic!("expected an identifier value, got {other:?}"),
    }
}

/// The four ways a candidate fails: a country that issues no IBANs, a length the registry does not
/// give, an alphabet the structure does not allow, and arithmetic that does not agree.
#[test]
fn rejects_iban_shaped_runs_that_do_not_check_out() {
    // Invalid check digits, from the `no/iban.py` and `es/iban.py` docstrings.
    rejects("IBAN NO92 8601 1117 947");
    rejects("IBAN ES12 1234 1234 1612 3456 7890");
    // The right number of characters for nobody: GB is 22.
    rejects("IBAN GB82WEST1234569876543");
    // XX is not a country that issues IBANs.
    rejects("IBAN XX82WEST12345698765432");
    // GB's structure is four letters then fourteen digits; this has digits where letters belong.
    rejects("IBAN GB8212ST12345698765432");
}

/// A rejected candidate is not the end of the story: an IBAN followed immediately by a digit run
/// over-matches into something that fails on length, and the account number inside it survives.
#[test]
fn an_iban_survives_a_digit_run_glued_to_its_end() {
    let scanner = support::scanner();
    let entities = scanner.scan("IBAN GB82WEST123456987654321234 booked", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].text, "GB82WEST12345698765432");
}

#[test]
fn accepts_bics_beside_a_label() {
    for (text, compact) in [
        ("SWIFT AGRIFRPP882", "AGRIFRPP882"),
        ("BIC: AGRIFRPP", "AGRIFRPP"),
        ("beneficiary bank DEUTDEFF500", "DEUTDEFF500"),
    ] {
        let (scheme, found, country) = identifier(text);
        assert_eq!(scheme, "bic");
        assert_eq!(found, compact);
        assert_eq!(country.as_deref(), Some(&compact[4..6]));
    }
}

#[test]
fn a_bic_reports_that_it_has_no_checksum() {
    let scanner = support::scanner();
    let entities = scanner.scan("BIC AGRIFRPP", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert!(entities[0].flags.contains(&Flag::NoChecksum));
    assert!((entities[0].confidence - 0.95).abs() < f32::EPSILON);
}

/// ISO 9362 has no check digit, so the guards are the length, the country code and the label. An
/// ordinary capitalised word carries a country code often enough that the label is what makes the
/// rule usable: DOCUMENT has ME in positions five and six, CUSTOMER has OM.
#[test]
fn rejects_code_shaped_words_and_malformed_codes() {
    rejects("SECTION 4: DOCUMENT CUSTOMER RETAINED");
    // A cue is present, but nine characters is not a length ISO 9362 defines.
    rejects("BIC AGRIFRPP8");
    // A cue is present, but ZZ is not a country.
    rejects("BIC AGRIZZPP");
    // No cue anywhere near it.
    rejects("the header read AGRIFRPP and nothing else");
}
