//! Sums of money: the code, the symbol, and the separators between the digits.
//!
//! The separator fixtures are the documented examples from `vendored/reference/price-parser/`,
//! whose `parse_number` docstring is where this arithmetic comes from. The subset covers the four
//! shapes that actually occur — grouped-and-fractioned in both conventions, space grouping, a bare
//! integer, and the single-separator case where both readings survive — plus the near-misses that
//! reach a money pattern and must not become money: version strings, page ranges and rates.

mod support;

use regex_entity_scanner::model::{Flag, Value};

/// The currency, the scaled integer and the exponent of the single entity in `text`.
fn money(text: &str) -> (String, String, u8, Vec<Flag>) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert_eq!(entities.len(), 1, "{text:?} produced {entities:?}");
    match &entities[0].value {
        Value::Money {
            currency,
            amount_minor,
            exponent,
        } => (
            currency.clone(),
            amount_minor.clone(),
            *exponent,
            entities[0].flags.clone(),
        ),
        other => panic!("expected a money value, got {other:?}"),
    }
}

fn rejects(text: &str) {
    let scanner = support::scanner();
    let entities = scanner.scan(text, 0);
    assert!(entities.is_empty(), "{text:?} produced {entities:?}");
}

#[test]
fn accepts_a_code_in_front_of_and_behind_its_amount() {
    for (text, expected) in [
        ("the invoice is EUR 1.234,56 net", "123456"),
        ("the invoice is 1,234.56 USD net", "123456"),
        ("the invoice is USD 1 234,56 net", "123456"),
        ("the invoice is EUR 1200 net", "120000"),
    ] {
        let (_, minor, exponent, flags) = money(text);
        assert_eq!(minor, expected, "{text:?}");
        assert_eq!(exponent, 2);
        assert!(flags.is_empty(), "{text:?} flagged {flags:?}");
    }
}

/// The exponent is the currency's own, and it is the only thing that decides what the digits mean.
/// The yen has no minor unit and the dinar has three, so the same digits scale differently.
#[test]
fn the_scale_comes_from_the_currency_not_from_the_digits() {
    assert_eq!(money("a fee of JPY 1200 was charged").1, "1200");
    assert_eq!(money("a fee of JPY 1200 was charged").2, 0);
    assert_eq!(money("a fee of BHD 1,234.567 was charged").1, "1234567");
    assert_eq!(money("a fee of BHD 1,234.567 was charged").2, 3);
}

/// The one shape both conventions claim. It is read as a thousands group, and the flag says the
/// reading was a choice rather than a fact.
#[test]
fn a_single_separator_before_three_digits_is_reported_as_inferred() {
    let (code, minor, _, flags) = money("the total was EUR 1.234 exactly");
    assert_eq!(code, "EUR");
    assert_eq!(minor, "123400");
    assert!(flags.contains(&Flag::SeparatorInferred), "{flags:?}");
}

#[test]
fn a_symbol_that_names_one_currency_is_not_flagged() {
    let (code, minor, _, flags) = money("the price is €1.234,56 today");
    assert_eq!(code, "EUR");
    assert_eq!(minor, "123456");
    assert!(flags.is_empty(), "{flags:?}");
}

/// A dollar sign is written for twenty-nine currencies. The rule names the most widely used one and
/// says, in the contract's own vocabulary, that it could not tell.
#[test]
fn a_shared_symbol_names_the_common_currency_and_flags_it() {
    let (code, minor, _, flags) = money("the price is $1,200.00 today");
    assert_eq!(code, "USD");
    assert_eq!(minor, "120000");
    assert!(flags.contains(&Flag::AmbiguousCurrency), "{flags:?}");
}

/// The capitals that distinguish one dollar from another are part of the symbol where somebody
/// writes them that way, and part of the previous word where they are not.
#[test]
fn capitals_in_front_of_a_sign_are_part_of_the_symbol_only_when_they_are_one() {
    let scanner = support::scanner();

    let (code, minor, _, _) = money("the price is R$49,90 today");
    assert_eq!(code, "BRL");
    assert_eq!(minor, "4990");

    let entities = scanner.scan("the price is US$40 today", 0);
    assert_eq!(entities.len(), 1, "{entities:?}");
    assert_eq!(entities[0].text, "$40", "{entities:?}");
}

/// The negatives are the point. Every one of these reaches a money pattern and none of them is a
/// sum of money.
#[test]
fn rejects_the_shapes_that_are_not_money() {
    for text in [
        // A version string: the groups are not thousands groups.
        "upgraded to USD 1.2.3 yesterday",
        // A page range: a separator on the boundary means the number continues.
        "see USD 120-134 for the schedule",
        // A rate with more decimals than the currency has minor units.
        "converted at USD 1.2345 per unit",
        // Three capitals that are not a currency at all.
        "the ISO 9001 audit is booked",
        // A code that is also a word, with nothing about money nearby.
        "TOP 40 singles of the year",
        // The amount runs into another token.
        "part number EUR 1200X is out of stock",
    ] {
        rejects(text);
    }
}

/// A word-like code is admitted when the sentence is about money, which is the whole difference
/// between the Tongan paʻanga and a chart position.
#[test]
fn a_word_like_code_needs_a_word_about_money() {
    let (code, minor, _, _) = money("the price paid was TOP 1200 for the lot");
    assert_eq!(code, "TOP");
    assert_eq!(minor, "120000");
}
