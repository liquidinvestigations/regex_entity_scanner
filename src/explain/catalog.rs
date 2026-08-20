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
    RuleDoc {
        rule_id: "date.rfc2822",
        entity_type: EntityType::Date,
        title: "Mail header date",
        matches: "The date line of an email or news message — Tue, 04 Mar 2021 09:12:00 +0200. \
                  Written by mail software rather than by people, so it is consistently spelled, \
                  and it carries the day of the week alongside the date.",
        standards: &[
            "RFC 5322 §3.3 — Internet Message Format, the date and time specification",
            "RFC 2822 §3.3 — the previous revision, whose obsolete zone abbreviations are still \
             emitted",
        ],
        checks: &[
            "the day of the week agrees with the calendar date, which a mangled or fabricated \
             header rarely does",
            "the date exists in the proleptic Gregorian calendar, and the clock is a real time of \
             day",
            "the year falls inside a plausibility window",
            "the zone is a numeric offset no further than eighteen hours from UTC, or one of the \
             North American abbreviations the obsolete grammar allows",
            "nothing alphanumeric runs into the match from either side",
        ],
        not_checked: &[
            "whether the message exists, was sent, or was sent when it says",
            "which named time zone the offset belongs to — many zones share an offset at different \
             times of year",
        ],
        authorities: &[Authority {
            name: "Internet Engineering Task Force",
            role: "publishes the Internet Message Format specification",
            url: "https://www.ietf.org/",
        }],
        ftm: FtmMapping {
            schema: "Analyzable",
            property: "res:dateMentioned",
            note: "FollowTheMoney has Document.date for the date a document carries as its own, \
                   and no property for a date mentioned in its text, so the mention is a local \
                   extension alongside the other Analyzable mention properties.",
        },
        references: &[
            Reference {
                title: "RFC 5322 §3.3",
                url: "https://www.rfc-editor.org/rfc/rfc5322#section-3.3",
                note: "the grammar this rule matches",
            },
            Reference {
                title: "RFC 5322 §4.3",
                url: "https://www.rfc-editor.org/rfc/rfc5322#section-4.3",
                note: "the obsolete zone abbreviations and what they mean",
            },
        ],
    },
    RuleDoc {
        rule_id: "date.clf",
        entity_type: EntityType::Date,
        title: "Web server log timestamp",
        matches: "The bracketed timestamp of the Common Log Format — [04/Mar/2021:09:12:00 +0200] \
                  — emitted by Apache, nginx and everything that imitates them. The brackets and \
                  the colon between date and clock make it self-identifying, which is why it needs \
                  no cue word.",
        standards: &[
            "NCSA Common Log Format, as implemented by the Apache HTTP Server mod_log_config \
             %t directive",
        ],
        checks: &[
            "the surrounding brackets are present, which is what distinguishes the format from a \
             slashed date somebody wrote by hand",
            "the date exists in the proleptic Gregorian calendar, and the clock is a real time of \
             day",
            "the year falls inside a plausibility window",
            "the zone is a numeric offset no further than eighteen hours from UTC",
        ],
        not_checked: &[
            "whether the request being logged happened, or happened then",
            "which named time zone the offset belongs to",
        ],
        authorities: &[Authority {
            name: "The Apache Software Foundation",
            role: "documents the log format the rest of the ecosystem copies",
            url: "https://httpd.apache.org/docs/current/mod/mod_log_config.html",
        }],
        ftm: FtmMapping {
            schema: "Analyzable",
            property: "res:dateMentioned",
            note: "FollowTheMoney has Document.date for the date a document carries as its own, \
                   and no property for a date mentioned in its text, so the mention is a local \
                   extension alongside the other Analyzable mention properties.",
        },
        references: &[Reference {
            title: "Apache mod_log_config",
            url: "https://httpd.apache.org/docs/current/mod/mod_log_config.html",
            note: "the %t directive that produces this timestamp",
        }],
    },
    RuleDoc {
        rule_id: "date.iso_week",
        entity_type: EntityType::Date,
        title: "ISO week date",
        matches: "A day named by its ISO week and weekday — 2021-W09-4, or 2021W094 in the basic \
                  spelling. Common in manufacturing, logistics and payroll data, where the week is \
                  the planning unit.",
        standards: &["ISO 8601 §4.1.4 — week date representations"],
        checks: &[
            "the week exists in that ISO year, so week 53 is accepted only in the years that have \
             one",
            "the weekday is 1 to 7 counted from Monday, as ISO numbers them",
            "the year falls inside a plausibility window",
            "nothing alphanumeric runs into the match from either side, which is what keeps the \
             shape out of serial numbers",
        ],
        not_checked: &[
            "the time of day, which the format does not carry",
            "that the ISO year is the calendar year — the first days of January can belong to the \
             previous ISO year, and the value is the calendar date it resolves to",
        ],
        authorities: &[Authority {
            name: "International Organization for Standardization",
            role: "publishes ISO 8601",
            url: "https://www.iso.org/iso-8601-date-and-time-format.html",
        }],
        ftm: FtmMapping {
            schema: "Analyzable",
            property: "res:dateMentioned",
            note: "FollowTheMoney has Document.date for the date a document carries as its own, \
                   and no property for a date mentioned in its text, so the mention is a local \
                   extension alongside the other Analyzable mention properties.",
        },
        references: &[Reference {
            title: "ISO 8601 at ISO",
            url: "https://www.iso.org/iso-8601-date-and-time-format.html",
            note: "the standard that defines the week numbering",
        }],
    },
    RuleDoc {
        rule_id: "bank.iban",
        entity_type: EntityType::BankAccount,
        title: "International Bank Account Number",
        matches: "An account number in the international form: a country code, two check digits \
                  and the national account number, written either compactly or in the groups of \
                  four people use on invoices. It is the strongest bank identifier in an \
                  investigative corpus, because it names one account at one institution.",
        standards: &[
            "ISO 13616 — International Bank Account Number",
            "ISO 7064 Mod 97, 10 — the check character system it uses",
        ],
        checks: &[
            "the country code has an entry in the IBAN registry, so a country that issues no IBANs \
             is a rejection",
            "the length is exactly what the registry gives for that country",
            "every position matches the registry's structure — digits where it says digits, \
             letters where it says letters",
            "ISO 7064 mod-97-10 over the rearranged form, which is what makes the country code and \
             the check digits part of the arithmetic",
            "nothing alphanumeric runs into the match from the left",
        ],
        not_checked: &[
            "whether the account exists, is open, or belongs to whoever the text says",
            "the national check digits some countries embed inside their own account number, which \
             are a separate scheme per country",
            "that the bank code names a bank that still trades under that code",
        ],
        authorities: &[
            Authority {
                name: "SWIFT",
                role: "is the registration authority for ISO 13616 and publishes the IBAN registry",
                url: "https://www.swift.com/standards/data-standards/iban-international-bank-account-number",
            },
            Authority {
                name: "International Organization for Standardization",
                role: "publishes ISO 13616 and ISO 7064",
                url: "https://www.iso.org/standard/81090.html",
            },
        ],
        ftm: FtmMapping {
            schema: "BankAccount",
            property: "iban",
            note: "FollowTheMoney's BankAccount schema takes the IBAN as its own typed property \
                   and uses it as a caption, which is exactly this value.",
        },
        references: &[
            Reference {
                title: "IBAN registry",
                url: "https://www.swift.com/standards/data-standards/iban-international-bank-account-number",
                note: "the per-country length and structure this rule validates against",
            },
            Reference {
                title: "IBAN checksum validation",
                url: "https://en.wikipedia.org/wiki/International_Bank_Account_Number#Validating_the_IBAN",
                note: "the rearrangement and the mod-97 arithmetic, worked through",
            },
        ],
    },
    RuleDoc {
        rule_id: "bank.bic",
        entity_type: EntityType::BankAccount,
        title: "Business Identifier Code (SWIFT code)",
        matches: "The code that names a financial institution rather than an account — four \
                  letters of institution, two of country, two of location, and an optional branch. \
                  It is what a payment instruction carries beside the account number.",
        standards: &["ISO 9362 — Business identifier code (BIC)"],
        checks: &[
            "the length is eight or eleven characters, the only two ISO 9362 defines",
            "positions five and six are an ISO 3166-1 alpha-2 country code that exists",
            "one of the words BIC, SWIFT, SWIFTBIC, IBAN, bank, beneficiary or correspondent \
             appears within 48 bytes either side",
            "nothing alphanumeric runs into the match from either side",
        ],
        not_checked: &[
            "any check digit — ISO 9362 defines none, so the structure and the label beside it are \
             the whole of what can be verified",
            "that the code is registered, or that the institution it names still exists",
            "that the branch code is one the institution actually uses",
        ],
        authorities: &[Authority {
            name: "SWIFT",
            role: "is the registration authority for ISO 9362 and allocates the codes",
            url: "https://www.swift.com/standards/data-standards/bic-business-identifier-code",
        }],
        ftm: FtmMapping {
            schema: "BankAccount",
            property: "bic",
            note: "FollowTheMoney's BankAccount schema carries the institution's BIC beside the \
                   account number, which is where a code found in a payment instruction belongs.",
        },
        references: &[
            Reference {
                title: "BIC at SWIFT",
                url: "https://www.swift.com/standards/data-standards/bic-business-identifier-code",
                note: "the structure and the registration process",
            },
            Reference {
                title: "SWIFT BIC search",
                url: "https://www.swift.com/bsl/",
                note: "where a code can be looked up against the register",
            },
        ],
    },
    RuleDoc {
        rule_id: "company.lei",
        entity_type: EntityType::CompanyId,
        title: "Legal Entity Identifier",
        matches: "The twenty-character code that identifies a party to a financial transaction — \
                  four characters naming the unit that issued it, thirteen identifying the \
                  organisation, and two check digits. It is the closest thing to a global company \
                  key, and it resolves to a named legal entity with a jurisdiction and an address.",
        standards: &[
            "ISO 17442 — Legal entity identifier (LEI)",
            "ISO 7064 Mod 97, 10 — the check character system it uses",
        ],
        checks: &[
            "ISO 7064 mod-97-10 over all twenty characters, with letters counting as their base-36 \
             value",
            "the length is exactly twenty, and the last two positions are digits",
            "nothing alphanumeric runs into the match from either side, which is what keeps \
             twenty characters of a hash from being read as a code",
        ],
        not_checked: &[
            "whether the identifier was ever issued — the arithmetic agrees for numbers GLEIF has \
             never allocated",
            "whether the registration is current: an LEI can lapse, and the code stays valid \
             arithmetic",
            "which company it names — the card links to GLEIF's search rather than resolving it",
        ],
        authorities: &[
            Authority {
                name: "Global Legal Entity Identifier Foundation",
                role: "operates the LEI system and publishes the full population as open data",
                url: "https://www.gleif.org/",
            },
            Authority {
                name: "International Organization for Standardization",
                role: "publishes ISO 17442",
                url: "https://www.iso.org/standard/78829.html",
            },
        ],
        ftm: FtmMapping {
            schema: "LegalEntity",
            property: "leiCode",
            note: "FollowTheMoney defines leiCode on the abstract LegalEntity, which every \
                   concrete company or organisation schema inherits.",
        },
        references: &[
            Reference {
                title: "GLEIF search",
                url: "https://search.gleif.org/",
                note: "the public register, where a code resolves to the entity it names",
            },
            Reference {
                title: "ISO 17442 at ISO",
                url: "https://www.iso.org/standard/78829.html",
                note: "the standard that defines the structure and the check digits",
            },
        ],
    },
    RuleDoc {
        rule_id: "security.isin",
        entity_type: EntityType::Security,
        title: "International Securities Identification Number",
        matches: "The twelve-character code that names a tradable instrument worldwide — a \
                  two-letter prefix, a nine-character national identifier and a check digit. It is \
                  the join key between a holding in one filing and the same holding in another.",
        standards: &[
            "ISO 6166 — Securities and related financial instruments, international securities \
             identification numbering system",
        ],
        checks: &[
            "the Luhn check digit over the letter-expanded form, so the prefix takes part in the \
             arithmetic",
            "the prefix is an ISO 3166-1 country, or one of the allocations ISO 6166 makes to \
             substitute agencies and to internationally cleared securities",
            "the length is exactly twelve and the last position is a digit",
            "nothing alphanumeric runs into the match from either side",
        ],
        not_checked: &[
            "whether the instrument exists, is listed, or is still traded",
            "the national identifier inside it, which may be a CUSIP or a SEDOL with its own check \
             digit that this rule does not separately verify",
            "that the prefix says where the issuer is — it says which agency allocated the number",
        ],
        authorities: &[
            Authority {
                name: "Association of National Numbering Agencies",
                role: "is the registration authority for ISO 6166 and coordinates the national \
                       numbering agencies",
                url: "https://www.anna-web.org/",
            },
            Authority {
                name: "International Organization for Standardization",
                role: "publishes ISO 6166",
                url: "https://www.iso.org/standard/78502.html",
            },
        ],
        ftm: FtmMapping {
            schema: "Security",
            property: "isin",
            note: "FollowTheMoney's Security schema takes the ISIN as a typed property and uses it \
                   as the entity's caption.",
        },
        references: &[Reference {
            title: "ANNA, the ISIN registration authority",
            url: "https://www.anna-web.org/standards/isin-iso-6166/",
            note: "how the numbers are allocated and by whom",
        }],
    },
    RuleDoc {
        rule_id: "security.cusip",
        entity_type: EntityType::Security,
        title: "CUSIP number",
        matches: "The nine-character code used for securities issued in the United States and \
                  Canada: six characters of issuer, two of issue and a check digit. It is a bare \
                  alphanumeric token, which is why it is only accepted beside a word that says \
                  what it is.",
        standards: &["CUSIP, as administered by CUSIP Global Services under ANSI X9.6"],
        checks: &[
            "the check digit, a Luhn variant with alternating weights of one and two",
            "one of the words CUSIP, SEDOL, ISIN, ticker, security or CIK appears within 48 bytes \
             either side",
            "the length is exactly nine and the last position is a digit",
            "nothing alphanumeric runs into the match from either side",
        ],
        not_checked: &[
            "whether the security exists or was ever issued",
            "the extended alphabet: CUSIPs containing *, @ or # are used for some private \
             placements and are not matched",
            "whether the issuer prefix belongs to the company the surrounding text names",
        ],
        authorities: &[Authority {
            name: "CUSIP Global Services",
            role: "allocates CUSIP numbers and operates the register",
            url: "https://www.cusip.com/",
        }],
        ftm: FtmMapping {
            schema: "Security",
            property: "res:cusip",
            note: "FollowTheMoney's Security schema has isin, ticker and figiCode but no CUSIP \
                   property, so this is a local extension in the res: namespace.",
        },
        references: &[Reference {
            title: "CUSIP Global Services",
            url: "https://www.cusip.com/",
            note: "the administrator, and where a number can be looked up",
        }],
    },
    RuleDoc {
        rule_id: "security.sedol",
        entity_type: EntityType::Security,
        title: "SEDOL number",
        matches: "The seven-character code the London Stock Exchange assigns to securities traded \
                  in the United Kingdom and Ireland. Its alphabet has no vowels, which is the only \
                  self-identification it offers, so it is accepted only beside a word that says \
                  what it is.",
        standards: &["SEDOL, the Stock Exchange Daily Official List numbering scheme"],
        checks: &[
            "the check digit, a weighted sum with weights 1, 3, 1, 7, 3, 9 modulo ten",
            "the alphabet excludes the vowels, which is written into the pattern itself",
            "numbers issued since 2004 begin with a letter and older ones are entirely numeric, so \
             a digit followed by letters is neither",
            "one of the words CUSIP, SEDOL, ISIN, ticker, security or CIK appears within 48 bytes \
             either side",
            "nothing alphanumeric runs into the match from either side",
        ],
        not_checked: &[
            "whether the security exists or is still listed",
            "which instrument it names — resolution needs the exchange's own data",
        ],
        authorities: &[Authority {
            name: "London Stock Exchange",
            role: "assigns SEDOL numbers and publishes the masterfile",
            url: "https://www.lseg.com/en/data-analytics/financial-data/identifiers/sedol",
        }],
        ftm: FtmMapping {
            schema: "Security",
            property: "res:sedol",
            note: "FollowTheMoney's Security schema has isin, ticker and figiCode but no SEDOL \
                   property, so this is a local extension in the res: namespace.",
        },
        references: &[Reference {
            title: "SEDOL masterfile",
            url: "https://www.lseg.com/en/data-analytics/financial-data/identifiers/sedol",
            note: "the issuing authority and the scheme's own documentation",
        }],
    },
    RuleDoc {
        rule_id: "vessel.imo",
        entity_type: EntityType::Vessel,
        title: "IMO ship identification number",
        matches: "The seven-digit number that identifies a ship's hull for its whole life, \
                  regardless of how often it is renamed, reflagged or resold. That permanence is \
                  what makes it the anchor for sanctions and ownership work, where the name on the \
                  stern is the least stable thing about a vessel.",
        standards: &[
            "IMO Resolution A.1078(28) — IMO ship identification number scheme",
            "ISO 8712 — Ships and marine technology, IMO ship identification number",
        ],
        checks: &[
            "the check digit, a weighted sum of the first six digits with weights seven down to \
             two, modulo ten",
            "either the literal marker IMO stands immediately before the digits, or one of the \
             words IMO, vessel, ship, tanker, hull or flag appears within 48 bytes either side",
            "the run is exactly seven digits and nothing alphanumeric touches it on either side",
        ],
        not_checked: &[
            "whether the ship exists, is still in class, or was ever registered — many hulls carry \
             a number whose check digit does not agree, and those are rejected here",
            "the ship's current name, flag or owner, none of which the number encodes",
        ],
        authorities: &[Authority {
            name: "International Maritime Organization",
            role: "owns the scheme; IHS Markit allocates the numbers on its behalf",
            url: "https://www.imo.org/",
        }],
        ftm: FtmMapping {
            schema: "Vessel",
            property: "imoNumber",
            note: "FollowTheMoney's Vessel schema defines imoNumber and features it in the caption, \
                   which is exactly this number.",
        },
        references: &[Reference {
            title: "IMO ship identification number scheme",
            url: "https://www.imo.org/en/OurWork/MSAS/Pages/IMO-identification-number-scheme.aspx",
            note: "the scheme, the allocation process and the check-digit rule",
        }],
    },
    RuleDoc {
        rule_id: "vessel.mmsi",
        entity_type: EntityType::Vessel,
        title: "MMSI (Maritime Mobile Service Identity)",
        matches: "The nine-digit identity a ship's radio and AIS transponder broadcast. Unlike an \
                  IMO number it belongs to the station rather than the hull, so it changes when a \
                  ship is reflagged — which is itself the signal an investigator is often looking \
                  for.",
        standards: &[
            "ITU-R M.585 — Assignment and use of identities in the maritime mobile service",
        ],
        checks: &[
            "the first three digits are a Maritime Identification Digit triple the ITU has \
             actually allocated, which is also where the flag state comes from",
            "one of the words MMSI, AIS, call sign, callsign, vessel or ship appears within 48 \
             bytes either side",
            "the run is exactly nine digits and nothing alphanumeric touches it on either side",
        ],
        not_checked: &[
            "a check digit — an MMSI has none, which is why the match carries the no-checksum flag",
            "coast-station, group-call and craft-associated identities, where the MID sits at \
             another position; reading the wrong three digits as a flag state is worse than not \
             reading them, so those forms are not matched",
            "whether the station is licensed or the transponder is transmitting the truth",
        ],
        authorities: &[Authority {
            name: "International Telecommunication Union",
            role: "allocates Maritime Identification Digits to administrations",
            url: "https://www.itu.int/en/ITU-R/terrestrial/fmd/Pages/mid.aspx",
        }],
        ftm: FtmMapping {
            schema: "Vessel",
            property: "mmsi",
            note: "FollowTheMoney's Vessel schema defines mmsi for exactly this identity.",
        },
        references: &[Reference {
            title: "ITU Maritime Identification Digits",
            url: "https://www.itu.int/en/ITU-R/terrestrial/fmd/Pages/mid.aspx",
            note: "the allocated triples and the administration each belongs to",
        }],
    },
    RuleDoc {
        rule_id: "container.iso6346",
        entity_type: EntityType::CargoContainer,
        title: "ISO 6346 container number",
        matches: "The eleven-character number stencilled on an intermodal shipping container: \
                  three letters of owner code, a category letter, six digits of serial and a check \
                  digit. It is what ties a bill of lading to a physical box, which is why it \
                  matters for trade and sanctions-evasion work.",
        standards: &["ISO 6346 — Freight containers, coding, identification and marking"],
        checks: &[
            "the check digit, each character's value times a power of two, summed, modulo eleven \
             then modulo ten",
            "the fourth character is one of U, J, Z or R, the only equipment categories the \
             standard defines, which is what makes the format self-identifying",
            "single spaces inside the grouped written form are stripped before checking",
            "nothing alphanumeric runs into the match from either side",
        ],
        not_checked: &[
            "whether the owner code is registered with the Bureau International des Containers",
            "the size and type code, which is stencilled beside the number and is not part of it",
            "whether the container exists or what it holds",
        ],
        authorities: &[Authority {
            name: "Bureau International des Containers et du Transport Intermodal",
            role: "maintains the owner-code register the standard depends on",
            url: "https://www.bic-code.org/",
        }],
        ftm: FtmMapping {
            schema: "Analyzable",
            property: "res:containerMentioned",
            note: "FollowTheMoney has no schema for an intermodal container and no property for \
                   its number, so this is a local extension in the res: namespace, following the \
                   shape of its own ipMentioned and ibanMentioned properties.",
        },
        references: &[Reference {
            title: "BIC container code register",
            url: "https://www.bic-code.org/bic-codes/",
            note: "where an owner code can be resolved to a company",
        }],
    },
];
