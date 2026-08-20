//! Equipment identifiers and network addresses.
//!
//! The IMEI fixtures are the documented examples from `stdnum/imei.py` — the valid number, its
//! grouped spelling, and the counterpart whose check digit disagrees. The rest of the subset is
//! chosen for the failure modes these four rules actually have: a certificate fingerprint against
//! `device.mac`, a four-component release number against `network.ip`, and the English word "as"
//! against `network.asn`.

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
        Value::NetworkAddress {
            family, address, ..
        } => (family.clone(), address.clone()),
        other => panic!("expected an identifier or address value, got {other:?}"),
    }
}

fn rejects(text: &str) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert!(entities.is_empty(), "{text:?} produced {entities:?}");
}

#[test]
fn accepts_imeis_compact_and_grouped_beside_a_device_word() {
    assert_eq!(
        only_match("Handset IMEI 354178036859789 seized"),
        ("imei".to_string(), "354178036859789".to_string())
    );
    assert_eq!(
        only_match("device 35-417803-685978-9 registered"),
        ("imei".to_string(), "354178036859789".to_string())
    );
}

#[test]
fn rejects_fifteen_digit_runs_that_are_not_imeis() {
    // No word says what it is.
    rejects("Reference 354178036859789 in the ledger.");
    // The Luhn check digit disagrees.
    rejects("Handset IMEI 354178036859781 seized.");
}

#[test]
fn accepts_mac_addresses_in_each_written_form() {
    let scanner = support::scanner();
    let entities = scanner.scan("lease for 00:1B:44:11:3A:B7 expires", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].entity_type, EntityType::Device);
    match &entities[0].value {
        Value::Identifier { compact, parts, .. } => {
            assert_eq!(compact, "00:1b:44:11:3a:b7");
            assert_eq!(parts.get("oui").map(String::as_str), Some("001b44"));
        }
        other => panic!("expected an identifier value, got {other:?}"),
    }

    assert_eq!(
        only_match("interface 00-1b-44-11-3a-b7 down").1,
        "00:1b:44:11:3a:b7"
    );
    assert_eq!(
        only_match("switchport 001b.4411.3ab7 learned").1,
        "00:1b:44:11:3a:b7"
    );
}

/// Six consecutive octets of a twenty-octet fingerprint look exactly like an address; the
/// separator on either side is the only thing that says otherwise.
#[test]
fn rejects_hashes_and_fingerprints_as_mac_addresses() {
    rejects("SHA1 fingerprint 5E:FF:56:A2:AF:15:22:B6:C4:5F:6E:B4:38:96:1F:AE:07:5B:8C:F1");
    rejects("sha256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 verified");
    // Mixed separators are two adjacent fields, not one address.
    rejects("field 00:1B-44:11:3A:B7 logged");
}

#[test]
fn accepts_addresses_of_both_families_and_cidr_prefixes() {
    assert_eq!(
        only_match("block 203.0.113.42 at the edge"),
        ("ipv4".to_string(), "203.0.113.42".to_string())
    );
    assert_eq!(
        only_match("X-Forwarded-For: 2001:db8::8a2e:370:7334 accepted"),
        ("ipv6".to_string(), "2001:db8::8a2e:370:7334".to_string())
    );

    let scanner = support::scanner();
    let entities = scanner.scan("route 198.51.100.0/24 via the gateway", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    match &entities[0].value {
        Value::NetworkAddress {
            address,
            prefix_length,
            ..
        } => {
            assert_eq!(address, "198.51.100.0");
            assert_eq!(*prefix_length, Some(24));
        }
        other => panic!("expected an address value, got {other:?}"),
    }
}

/// A four-component dotted number is also how a release is written, and the arithmetic cannot
/// separate the two — only the words around it can.
#[test]
fn rejects_version_numbers_and_impossible_octets() {
    rejects("upgraded to firmware version 10.0.0.4 last night");
    rejects("host 10.0.0.256 unreachable");
    rejects("route 198.51.100.0/64 via the gateway");
}

#[test]
fn accepts_autonomous_system_numbers_behind_their_prefix() {
    assert_eq!(
        only_match("announced by AS64512 since March"),
        ("asn".to_string(), "AS64512".to_string())
    );
    assert_eq!(
        only_match("peer ASN 15169 upstream"),
        ("asn".to_string(), "AS15169".to_string())
    );
}

#[test]
fn rejects_numbers_without_the_autonomous_system_prefix() {
    rejects("carried 64512 tonnes of cargo");
    // Beyond the 32-bit space.
    rejects("route from AS9999999999 rejected");
}
