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

/// One valid number per member state, taken from the docstring of the `python-stdnum` module the
/// arithmetic is ported from. This is the test that the table is complete: twenty-seven states,
/// twenty-seven algorithms, and Greece under the prefix `EL` that the tax system uses rather than
/// the `GR` that ISO 3166-1 does.
#[test]
fn accepts_a_documented_vat_number_for_every_member_state() {
    let scanner = support::scanner();
    let table = [
        ("ATU13585627", "AT"),
        ("BE0428759497", "BE"),
        ("BG175074752", "BG"),
        ("CY10259033P", "CY"),
        ("CZ25123891", "CZ"),
        ("DE136695976", "DE"),
        ("DK13585628", "DK"),
        ("EE100594102", "EE"),
        ("EL094259216", "GR"),
        ("ESB64717838", "ES"),
        ("FI20774740", "FI"),
        ("FR23334175221", "FR"),
        ("HR33392005961", "HR"),
        ("HU12892312", "HU"),
        ("IE6433435F", "IE"),
        ("IT00743110157", "IT"),
        ("LT119511515", "LT"),
        ("LU15027442", "LU"),
        ("LV40003521600", "LV"),
        ("MT11679112", "MT"),
        ("NL004495445B01", "NL"),
        ("PL8567346215", "PL"),
        ("PT501964843", "PT"),
        ("RO18547290", "RO"),
        ("SE123456789701", "SE"),
        ("SI50223054", "SI"),
        ("SK2022749619", "SK"),
    ];
    assert_eq!(table.len(), 27, "every member state has a fixture");

    for (number, expected_country) in table {
        let text = format!("VAT {number} on the invoice");
        let entities = scanner.scan(&text, 0);
        let entity = entities
            .iter()
            .find(|entity| entity.rule_id == "company.vat_eu")
            .unwrap_or_else(|| panic!("{number} produced {entities:?}"));
        assert_eq!(entity.entity_type, EntityType::CompanyId);
        assert_eq!(entity.text, number);
        match &entity.value {
            Value::Identifier {
                scheme,
                compact,
                country,
                ..
            } => {
                assert_eq!(scheme, "vat");
                assert_eq!(compact, number);
                assert_eq!(country.as_deref(), Some(expected_country), "for {number}");
            }
            other => panic!("expected an identifier value for {number}, got {other:?}"),
        }
    }
}

/// The alternative spellings the member states' own modules accept: the historical short forms
/// that the register zero-fills, the letter-bearing bodies, and the personal numbers that several
/// states admit as a VAT number in their own right.
#[test]
fn accepts_the_alternative_spellings_the_member_states_allow() {
    let scanner = support::scanner();
    for (number, canonical) in [
        // Nine digits is the pre-2005 Belgian form; the register carries a leading zero.
        ("BE428759497", "BE0428759497"),
        // Eight digits is the older Greek form, zero-filled the same way.
        ("EL94259216", "EL094259216"),
        // Ireland writes the 2013 two-letter form and the old form whose second character is a
        // letter, and both check out through the same alphabet.
        ("IE6433435OA", "IE6433435OA"),
        ("IE8D79739I", "IE8D79739I"),
        // A French key with a letter in it is checked differently from an all-numeric one.
        ("FRK7399859412", "FRK7399859412"),
        ("FR4Z123456782", "FR4Z123456782"),
        // The current Dutch form checks under ISO 7064 rather than through a BSN.
        ("NL002455799B11", "NL002455799B11"),
        // Lithuania's twelve-digit temporary registration.
        ("LT100001919017", "LT100001919017"),
        // Czechia's birth number and its nine-digit special case.
        ("CZ7103192745", "CZ7103192745"),
        ("CZ640903926", "CZ640903926"),
        // Latvia's personal codes, in the pre-2017 date form and the current one.
        ("LV16117519997", "LV16117519997"),
        ("LV32867300679", "LV32867300679"),
        // Romania's thirteen-digit personal number.
        ("RO1630615123457", "RO1630615123457"),
        // Spain's resident, foreign-national and residual forms.
        ("ES54362315K", "ES54362315K"),
        ("ESX5253868R", "ESX5253868R"),
        ("ESM1234567L", "ESM1234567L"),
    ] {
        let text = format!("VAT {number} on the invoice");
        let entities = scanner.scan(&text, 0);
        let entity = entities
            .iter()
            .find(|entity| entity.rule_id == "company.vat_eu")
            .unwrap_or_else(|| panic!("{number} produced {entities:?}"));
        match &entity.value {
            Value::Identifier { compact, .. } => assert_eq!(compact, canonical),
            other => panic!("expected an identifier value for {number}, got {other:?}"),
        }
    }
}

/// The three ways a VAT-shaped run is not a VAT number: the check digit disagrees, the body is the
/// wrong length for the state, or the prefix is not one a member state issues under.
#[test]
fn rejects_vat_shaped_runs_that_are_not_vat_numbers() {
    let scanner = support::scanner();
    for text in [
        // Documented invalid-checksum counterparts.
        "VAT ATU13585626",
        "VAT BE0431150351",
        "VAT DE136695978",
        "VAT MT11679113",
        "VAT PL8567346216",
        "VAT SE123456789101",
        "VAT SI50223055",
        "VAT CY10259033Z",
        "VAT IE6433435E",
        // Right arithmetic, wrong length for the state: Germany issues nine digits, not ten.
        "VAT DE1366959760",
        // Greece uses EL for VAT; the ISO 3166-1 code is not a VAT prefix.
        "VAT GR094259216",
        // Not a member state.
        "VAT CH136695976",
        // A Romanian body shorter than four digits: the register allocates them, but the shape
        // is one ordinary text produces and the single check digit does not carry it.
        "route RO51 closed",
        // A capitalised word that opens with a member-state prefix.
        "see ATTENTION and DEADLINE",
    ] {
        let entities = scanner.scan(text, 0);
        assert!(
            !entities
                .iter()
                .any(|entity| entity.rule_id == "company.vat_eu"),
            "{text:?} produced {entities:?}"
        );
    }
}
