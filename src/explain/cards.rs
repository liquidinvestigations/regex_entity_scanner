//! Per-rule shapers: what only this particular match can say.
//!
//! The catalogue supplies everything true of the rule. A shaper adds what is true of the match in
//! front of the reader — the country behind its top-level domain, the weekday its date fell on,
//! the register entry for its own identifier — and that is the half that makes a card worth
//! opening.
//!
//! A rule with no shaper still gets a card from the catalogue alone. That fallback is what keeps
//! documenting a new rule cheap: write the entry, ship a usable card, refine it later.

use crate::data::VendoredData;
use crate::explain::catalog::RuleDoc;
use crate::explain::{ExplainRequest, Explanation, Fact, Link};

pub fn build(doc: &'static RuleDoc, request: &ExplainRequest, data: &VendoredData) -> Explanation {
    let mut card = Explanation {
        rule_id: doc.rule_id.to_string(),
        entity_type: doc.entity_type,
        title: doc.title.to_string(),
        subtitle: String::new(),
        body: String::new(),
        facts: Vec::new(),
        references: doc
            .references
            .iter()
            .map(|reference| Link::new(reference.title, reference.url, reference.note))
            .collect(),
    };

    // The paragraph about this specific match, which is also where a shaper adjusts the title,
    // subtitle, facts and links.
    let specifics = match doc.rule_id {
        "date.iso8601" => date_iso(&mut card, request),
        "email.basic" => email(&mut card, request, data),
        _ => None,
    };

    if let Some(confidence) = request.confidence {
        card.facts
            .push(Fact::new("Confidence", format!("{confidence:.2}")));
    }
    card.body = body(doc, request, specifics);
    card
}

/// Assembles the long text. The order is fixed across rules so that a reader who has opened one
/// card knows where to look in the next: what this is, what this one says, what was verified, what
/// verification does not cover, who defines it, where to read more.
fn body(doc: &'static RuleDoc, request: &ExplainRequest, specifics: Option<String>) -> String {
    let mut blocks: Vec<String> = vec![doc.matches.to_string()];

    if let Some(text) = &request.text {
        blocks.push(format!("**Matched text:** `{text}`"));
    }
    if let Some(specifics) = specifics {
        blocks.push(specifics);
    }

    if !doc.checks.is_empty() {
        blocks.push(format!(
            "**What was checked**\n{}",
            bullets(doc.checks.iter().copied())
        ));
    }
    if !doc.not_checked.is_empty() {
        blocks.push(format!(
            "**What this does not prove**\n{}",
            bullets(doc.not_checked.iter().copied())
        ));
    }
    if !doc.standards.is_empty() {
        blocks.push(format!(
            "**Defined by**\n{}",
            bullets(doc.standards.iter().copied())
        ));
    }
    if !doc.authorities.is_empty() {
        blocks.push(format!(
            "**Authorities**\n{}",
            bullets(doc.authorities.iter().map(|authority| format!(
                "[{}]({}) — {}",
                authority.name, authority.url, authority.role
            )))
        ));
    }
    if !doc.references.is_empty() {
        blocks.push(format!(
            "**References**\n{}",
            bullets(doc.references.iter().map(|reference| format!(
                "[{}]({}) — {}",
                reference.title, reference.url, reference.note
            )))
        ));
    }

    blocks.join("\n\n")
}

fn bullets(items: impl Iterator<Item = impl std::fmt::Display>) -> String {
    items
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// ISO 8601: precision decides what to call it, and the calendar gives the reader the one thing the
/// digits do not — which day of the week it was.
fn date_iso(card: &mut Explanation, request: &ExplainRequest) -> Option<String> {
    let rfc3339 = request.value_str("rfc3339")?;
    let precision = request.value_str("precision").unwrap_or("day");
    let tz_known = request.value_bool("tz_known").unwrap_or(false);

    card.title = match precision {
        "day" => "ISO 8601 date",
        _ => "ISO 8601 timestamp",
    }
    .to_string();

    let date: jiff::civil::Date = rfc3339.get(..10)?.parse().ok()?;
    let weekday = weekday_name(date.weekday());
    let readable = format!(
        "{weekday}, {} {} {}",
        date.day(),
        month_name(date.month()),
        date.year()
    );

    let zone = match (tz_known, offset_of(rfc3339)) {
        (true, Some("Z")) => "UTC".to_string(),
        (true, Some(offset)) => format!("UTC{offset}"),
        _ => "no time zone stated".to_string(),
    };

    card.subtitle = format!("{readable} · {precision} precision · {zone}");
    card.facts.push(Fact::new("Canonical value", rfc3339));
    card.facts.push(Fact::new("Day of the week", weekday));
    card.facts.push(Fact::new("Precision", precision));
    card.facts.push(Fact::new("Time zone", zone.as_str()));
    let week = date.iso_week_date();
    card.facts.push(Fact::new(
        "ISO week",
        format!("{}-W{:02}", week.year(), week.week()),
    ));

    let mut specifics = format!("This is {readable}.");
    if !tz_known {
        specifics.push_str(
            " No offset was written, so the instant it refers to depends on where the document was \
             produced — the date is certain, the moment is not.",
        );
    }
    if request.text.as_deref().is_some_and(|text| text != rfc3339) {
        specifics.push_str(&format!(
            " It was written differently from the canonical form above; both spellings normalise to \
             `{rfc3339}`, which is what makes a date range query find them together."
        ));
    }
    Some(specifics)
}

/// Email: the domain is the interesting part, because it is the only part anything was verified
/// against. A country-code top-level domain also names a country, which is usually the single most
/// useful thing on the card.
fn email(card: &mut Explanation, request: &ExplainRequest, data: &VendoredData) -> Option<String> {
    let address = request.value_str("address")?;
    let local = request.value_str("local").unwrap_or_default();
    let domain = request.value_str("domain")?;
    let tld = domain.rsplit('.').next()?;

    let kind = tld_kind(tld, data);
    card.subtitle = match &kind.country {
        Some(country) => format!("{domain} · {country}"),
        None => format!("{domain} · {}", kind.description),
    };

    card.facts.push(Fact::new("Address", address));
    if !local.is_empty() {
        card.facts.push(Fact::new("Local part", local));
    }
    card.facts.push(Fact::new("Domain", domain));
    card.facts
        .push(Fact::new("Top-level domain", format!(".{tld}")));
    card.facts
        .push(Fact::new("Domain type", kind.description.as_str()));
    if let Some(country) = &kind.country {
        card.facts.push(Fact::new("Country", country.as_str()));
    }

    // The register entry for this specific domain: who sponsors it and under what policy.
    card.references.insert(
        0,
        Link::new(
            format!("IANA register entry for .{tld}"),
            format!("https://www.iana.org/domains/root/db/{tld}.html"),
            "who sponsors this top-level domain and under what policy",
        ),
    );

    let mut specifics = if local.is_empty() {
        format!("The address names a mailbox at `{domain}`.")
    } else {
        format!(
            "The local part `{local}` identifies a mailbox at `{domain}`. The domain is compared \
             case-insensitively and is stored lowercased; the local part is case-sensitive and is \
             left exactly as written."
        )
    };
    match &kind.country {
        Some(country) => specifics.push_str(&format!(
            " `.{tld}` is the country-code top-level domain for {country}, which says where the \
             domain is registered — not necessarily where its holder is."
        )),
        None => specifics.push_str(&format!(" `.{tld}` is a {}.", kind.description)),
    }
    Some(specifics)
}

struct TldKind {
    description: String,
    country: Option<String>,
}

/// A two-letter top-level domain is a country code, with the handful of historical exceptions that
/// do not match their ISO 3166 code.
fn tld_kind(tld: &str, data: &VendoredData) -> TldKind {
    let lower = tld.to_lowercase();

    if lower.starts_with("xn--") {
        return TldKind {
            description: "internationalised top-level domain".to_string(),
            country: None,
        };
    }

    if lower.len() == 2
        && lower
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        // `.uk` predates ISO 3166 assigning GB, and was never renamed.
        let alpha2 = match lower.as_str() {
            "uk" => "GB",
            other => other,
        };
        let country = data.territory_name(alpha2).map(str::to_string);
        return TldKind {
            description: "country-code top-level domain".to_string(),
            country,
        };
    }

    TldKind {
        description: "generic top-level domain".to_string(),
        country: None,
    }
}

/// The offset as written at the end of an RFC 3339 timestamp, if there is one.
fn offset_of(rfc3339: &str) -> Option<&str> {
    if rfc3339.ends_with('Z') {
        return Some("Z");
    }
    let time = rfc3339.split_once('T')?.1;
    let index = time.rfind(['+', '-'])?;
    Some(&time[index..])
}

fn weekday_name(weekday: jiff::civil::Weekday) -> &'static str {
    use jiff::civil::Weekday::*;
    match weekday {
        Monday => "Monday",
        Tuesday => "Tuesday",
        Wednesday => "Wednesday",
        Thursday => "Thursday",
        Friday => "Friday",
        Saturday => "Saturday",
        Sunday => "Sunday",
    }
}

fn month_name(month: i8) -> &'static str {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    MONTHS
        .get((month - 1).clamp(0, 11) as usize)
        .copied()
        .unwrap_or("")
}
