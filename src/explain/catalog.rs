//! What we know about each rule, written down once.
//!
//! This is static because it is knowledge about the rule, not about any particular match: the
//! standard that defines the format, the authority that administers it, what the validator verifies
//! and what it deliberately does not. It belongs next to the rule for the same reason a doc comment
//! does — it goes stale the moment it lives anywhere else.
//!
//! Every rule gets an entry. A rule with an entry and no shaper in [`super::cards`] still produces
//! a usable card from this alone, so the catalogue is the requirement and the shaper is the polish.
//!
//! `not_checked` deserves as much care as `checks`. A reader deciding whether to trust a match
//! needs to know that a validated address has never been delivered to, and that a valid check digit
//! says a number is well-formed and nothing whatever about whether it was ever issued.

use serde::Serialize;

use crate::model::EntityType;

#[derive(Debug, Serialize)]
pub struct RuleDoc {
    pub rule_id: &'static str,
    pub entity_type: EntityType,
    /// Human-readable name for the rule itself. A shaper may make it more specific for a given
    /// match — "ISO 8601 timestamp" rather than "ISO 8601 date and time".
    pub title: &'static str,
    /// One or two sentences: what shapes this rule matches, and why it is worth having.
    pub matches: &'static str,
    /// The standards the format is defined by, spelled the way a reader would search for them.
    pub standards: &'static [&'static str],
    /// What the validator verifies before a candidate is accepted.
    pub checks: &'static [&'static str],
    /// What acceptance does *not* prove.
    pub not_checked: &'static [&'static str],
    pub authorities: &'static [Authority],
    pub references: &'static [Reference],
    /// Where this extraction lands in FollowTheMoney, which is the schema an investigative
    /// consumer already has. Every rule carries one; the test suite fails otherwise.
    pub ftm: FtmMapping,
}

/// The FollowTheMoney schema and property an extraction feeds.
///
/// Where FollowTheMoney defines a property we use its exact name, and where the property sits on an
/// abstract parent — `amount` and `currency` on `Value`, `idNumber` and `leiCode` on `LegalEntity`
/// — the mapping names the parent, because that is where FollowTheMoney defines it and every
/// concrete schema a consumer builds inherits it.
#[derive(Debug, Serialize)]
pub struct FtmMapping {
    /// The schema, e.g. `BankAccount`.
    pub schema: &'static str,
    /// The property on it, e.g. `iban`. **Prefixed `res:` where FollowTheMoney has no property for
    /// this and we are extending its schema locally** — an extension that is written down is a
    /// mapping a consumer can implement, and one that is not is a surprise.
    pub property: &'static str,
    /// Why this is the right property, or why FollowTheMoney has none and an extension was needed.
    pub note: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Authority {
    pub name: &'static str,
    /// What this body actually does for this identifier: publishes the standard, runs the register,
    /// allocates the ranges.
    pub role: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Reference {
    pub title: &'static str,
    pub url: &'static str,
    pub note: &'static str,
}

pub fn lookup(rule_id: &str) -> Option<&'static RuleDoc> {
    RULE_DOCS.iter().find(|doc| doc.rule_id == rule_id)
}

pub fn all() -> &'static [RuleDoc] {
    RULE_DOCS
}

static RULE_DOCS: &[RuleDoc] = &[
    RuleDoc {
        rule_id: "date.iso8601",
        entity_type: EntityType::Date,
        title: "ISO 8601 date and time",
        matches: "A date written in the international machine-readable order — year, month, day, \
                  largest unit first — optionally followed by a time and a UTC offset. This is the \
                  form used by logs, exports, mail headers and databases, and it is unambiguous: \
                  unlike 03/04/2021, it cannot be read two ways.",
        standards: &[
            "ISO 8601 — Date and time representations for information interchange",
            "RFC 3339 — Date and Time on the Internet: Timestamps, the internet profile of ISO 8601",
        ],
        checks: &[
            "the date exists in the proleptic Gregorian calendar, so 30 February and 31 April are \
             rejected and leap years are respected",
            "the clock is a real time of day, so hour 24 and minute 61 are rejected",
            "the year falls inside a plausibility window, which is what keeps version strings and \
             identifier fragments out of the date facet",
            "no digit runs into the match from either side, so a date embedded in a longer number \
             is not treated as a date",
        ],
        not_checked: &[
            "whether the date is correct — only that it is a date that exists",
            "which time zone an offset belongs to: +02:00 is an offset from UTC, and many zones \
             share it at different times of year",
        ],
        authorities: &[
            Authority {
                name: "International Organization for Standardization",
                role: "publishes ISO 8601",
                url: "https://www.iso.org/iso-8601-date-and-time-format.html",
            },
            Authority {
                name: "Internet Engineering Task Force",
                role: "publishes RFC 3339, the profile used on the internet",
                url: "https://www.ietf.org/",
            },
        ],
        ftm: FtmMapping {
            schema: "Analyzable",
            property: "res:dateMentioned",
            note: "FollowTheMoney has Document.date for the date a document carries as its own, \
                   and no property for a date mentioned in its text, so the mention is a local \
                   extension alongside the other Analyzable mention properties.",
        },
        references: &[
            Reference {
                title: "RFC 3339",
                url: "https://www.rfc-editor.org/rfc/rfc3339",
                note: "the timestamp grammar this value is normalised to",
            },
            Reference {
                title: "ISO 8601 at ISO",
                url: "https://www.iso.org/iso-8601-date-and-time-format.html",
                note: "the standard itself",
            },
            Reference {
                title: "IANA time zone database",
                url: "https://www.iana.org/time-zones",
                note: "how offsets map to named zones, and why one offset is not one zone",
            },
        ],
    },
    RuleDoc {
        rule_id: "email.basic",
        entity_type: EntityType::Email,
        title: "Email address",
        matches: "An address in the ordinary local-part@domain form that people actually write. \
                  Full RFC 5322 syntax — quoted local parts, comments, folded whitespace — is \
                  deliberately not the target: it accepts a great deal nobody has ever typed, and \
                  on real documents the precision comes from checking the domain instead.",
        standards: &[
            "RFC 5321 — Simple Mail Transfer Protocol, which sets the length limits",
            "RFC 5322 — Internet Message Format, which defines the address grammar",
        ],
        checks: &[
            "the top-level domain is on IANA's list of delegated domains, which is what separates \
             an address from a file name such as sprite@2x.png",
            "the local part is addressable unquoted — no leading, trailing or doubled dot",
            "the local part and the domain are inside the RFC 5321 length limits of 64 and 255 \
             characters",
            "nothing addressable runs into the match from either side, so an address is not cut \
             out of a longer token",
        ],
        not_checked: &[
            "whether the domain exists, resolves, or accepts mail — no lookup of any kind is made",
            "whether the mailbox exists or the address was ever in use",
            "whether the address belongs to the person the surrounding text is about",
        ],
        authorities: &[Authority {
            name: "Internet Assigned Numbers Authority",
            role: "delegates top-level domains and publishes the authoritative list",
            url: "https://www.iana.org/domains/root/db",
        }],
        ftm: FtmMapping {
            schema: "Analyzable",
            property: "emailMentioned",
            note: "The property FollowTheMoney defines for an address found in a document's text, \
                   as opposed to LegalEntity.email, which asserts that the address belongs to the \
                   entity — something extraction cannot establish.",
        },
        references: &[
            Reference {
                title: "IANA Root Zone Database",
                url: "https://www.iana.org/domains/root/db",
                note: "every delegated top-level domain and who sponsors it",
            },
            Reference {
                title: "RFC 5321 §4.5.3.1",
                url: "https://www.rfc-editor.org/rfc/rfc5321#section-4.5.3.1",
                note: "the local-part and domain length limits applied here",
            },
            Reference {
                title: "RFC 5321 §2.4",
                url: "https://www.rfc-editor.org/rfc/rfc5321#section-2.4",
                note: "why the domain may be lowercased and the local part may not",
            },
        ],
    },
];
