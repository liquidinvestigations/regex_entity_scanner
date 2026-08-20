//! Company identifiers.
//!
//! The fixtures are the documented examples from the `python-stdnum` modules the arithmetic is
//! ported from — `stdnum/lei.py` for the valid code and its single-transposition counterpart. The
//! subset covers what the rule claims: the checksum, the fixed length, and the boundary guards that
//! keep twenty characters of a hash out of the facet.

mod support;

use regex_entity_scanner::model::{EntityType, Value};

#[test]
fn accepts_a_documented_lei() {
    let scanner = support::scanner();
    let entities = scanner.scan("counterparty LEI 213800KUD8LAJWSQ9D15 on file", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    let entity = &entities[0];
    assert_eq!(entity.entity_type, EntityType::CompanyId);
    assert_eq!(entity.rule_id, "company.lei");
    assert_eq!(entity.text, "213800KUD8LAJWSQ9D15");
    match &entity.value {
        Value::Identifier {
            scheme,
            compact,
            parts,
            ..
        } => {
            assert_eq!(scheme, "lei");
            assert_eq!(compact, "213800KUD8LAJWSQ9D15");
            assert_eq!(parts.get("lou").map(String::as_str), Some("2138"));
            assert_eq!(parts.get("check_digits").map(String::as_str), Some("15"));
        }
        other => panic!("expected an identifier value, got {other:?}"),
    }
}

/// One character changed is what the check digits exist to catch, and twenty characters cut out of
/// a longer token are not a code at all.
#[test]
fn rejects_lei_shaped_runs_that_do_not_check_out() {
    let scanner = support::scanner();
    for text in [
        // The documented invalid-checksum counterpart: A became X.
        "LEI 213800KUD8LXJWSQ9D15",
        // A run of the right alphabet, but longer than twenty.
        "LEI 213800KUD8LAJWSQ9D1500",
        "sha1 DA39A3EE5E6B4B0D3255BFEF95601890AFD80709",
    ] {
        let entities = scanner.scan(text, 0);
        assert!(entities.is_empty(), "{text:?} produced {entities:?}");
    }
}
