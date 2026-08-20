# `src/rules/`

One module per entity type. A rule is a candidate pattern plus a validator, and adding a type is a
pattern, a validator and a fixture — not a new pipeline.

## The inventory

| Module | Rules |
|---|---|
| `email.rs` | `email.basic` |
| `date_iso.rs` | `date.iso8601` |
| `date_machine.rs` | `date.rfc2822`, `date.clf`, `date.iso_week` |
| `bank.rs` | `bank.iban`, `bank.bic` |
| `company.rs` | `company.lei`, `company.vat_eu`, `company.vat_non_eu` |
| `security.rs` | `security.isin`, `security.cusip`, `security.sedol` |
| `maritime.rs` | `vessel.imo`, `vessel.mmsi`, `container.iso6346` |
| `device.rs` | `device.imei`, `device.mac` |
| `network.rs` | `network.ip`, `network.asn` |
| `extras.rs` | `vulnerability.cve`, `publication.doi`, `publication.orcid`, `message.rfc5322` |
| `coordinates.rs` | `coord.decimal`, `coord.dms`, `coord.plus_code` |
| `crypto.rs` | `crypto.ethereum`, `crypto.bitcoin` |
| `money.rs` | `money.iso_code`, `money.symbol` |
| `national_id.rs` | `natid.it_codice_fiscale`, `natid.es_nif_nie`, `natid.mx_curp`, `natid.in_pan` |

## A date rule ships only when the match fixes the whole day

Standing policy for the `date` type, and the reason there are four date rules rather than six: **a
date rule emits nothing unless the match itself determines year, month and day.**

Yearless formats are therefore not matched — a syslog line's `Mar  4 09:12:00` would have to have
its year invented, taken from document state this service does not hold, or left out of a value a
range query then cannot use. Bare `YYYYMMDD` is not matched either: it does fix the whole day, but
eight adjacent digits with a valid calendar reading is the noisiest date shape there is, and in an
identifier-heavy corpus most of them are not dates. It is the same policy that keeps `2021-W09` out
of `date.iso_week`, which names a week rather than a day.

## National identity numbers are detected, never decoded

Standing policy for the `national_id` type, and it has two halves.

**Scope.** A scheme ships only when its surface form identifies itself — a fixed letter-and-digit
pattern, a restricted check alphabet, or a holder-type position. A bare eleven-digit run with a
check digit is still a bare eleven-digit run, and a facet built out of those is a facet of invoice
numbers.

**Depth.** Several of these identifiers encode their holder's date of birth and sex in fixed
positions; the codice fiscale and the CURP both do. No validator reads those positions, no value
carries them, and each catalogue entry says so. Detecting that a document mentions an identity
number is a different act from deriving personal attributes out of it, and this service does only
the first.

## The rule-set version

`RULE_SET_VERSION` ships on every scan response and on `/health` and `/rules`, and it is what makes
the scope of a reindex computable: extraction is not idempotent across rule sets.

Bump it for any change to a candidate pattern, to a validator's accept or reject boundary, to the
value a rule normalises to, or to the rule inventory. Do not bump it for card text, a doc comment or
a catalogue entry — none of those change what a document yields.

## The contract

`candidate_pattern` is compiled into the prefilter alongside every other rule's, so it must be
expressible in a linear-time engine. Where an upstream pattern relies on lookaround, the guard is
stripped here and re-imposed in `validate`: `(?<!\d)` is exactly "the preceding byte is not a
digit", which is a one-byte check on a candidate rather than an assertion the engine has to carry.

A pattern also carries no `^`, `$` or `\b`, and the prefilter refuses to compile one that does. The
scan loop re-runs a rule's pattern over a **slice** of the fragment when it looks inside a rejected
candidate, and an anchor or a word boundary means something different there. Boundary conditions
belong in the validator, which holds the whole fragment either way.

`validate` returns `None` to reject, or a `Verdict` carrying the canonical value, a confidence, any
flags, and possibly a narrowed span — trailing punctuation that the pattern swallowed is trimmed
here, where it is cheap to reason about, instead of in the pattern where it is not.

## Guards that earn their keep

In rough order of how much noise they remove per line of code:

1. **Structural validity** — a calendar that has the day, a check digit that agrees. This alone
   removes the bulk of numeric false positives.
2. **Plausibility windows** — a year range, an amount magnitude. Kills version strings and
   identifier fragments.
3. **Adjacent-byte guards** — the stripped lookarounds. A date wedged between digits is part of a
   longer number.
4. **List membership** — the IANA TLD list, the IBAN registry, the ISO 3166-1 country codes,
   airport codes, MMSI flag prefixes. Cheap, and decisive for the types that have no checksum.
   `VendoredData::is_country_code` is the country test: CLDR's territory list also holds groupings
   and placeholders such as `EU` and `ZZ`, and an identifier that encodes a country encodes a real
   one.
5. **Ambiguous-lexeme rules** — surface forms that collide with ordinary words and need stronger
   context before they count.

## Shared machinery

`checksum.rs` is the check-digit arithmetic — `luhn`, `damm`, `weighted_mod`, and `iso7064`'s
`mod_97_10`, `mod_11_2` and `mod_11_10` — ported from `python-stdnum`, each with the valid and
invalid pair from its upstream documentation as a test. The arithmetic is not invented here; a
divergence from the reference shows up as a failing test rather than a quietly wrong facet.

`context.rs` adds `Candidate::has_cue`, which answers whether one of a rule's cue words appears
within a window either side of the candidate, case-insensitively and on ASCII word boundaries. A
check digit on a bare digit run is a one-in-ten filter, so the formats that are nothing but a token
and a check digit are admitted on a cue word as well.

## Confidence

Confidence is what a consumer thresholds on at index time, so it has to mean something consistent
across rules, which means picking a number off a fixed ladder rather than forming an opinion:

| Value | What the validator was able to do |
|---|---|
| 0.99 | Check digit verified **and** the surface form is self-identifying — a country prefix, a fixed alphabet, a literal marker |
| 0.97 | Check digit verified on a bare token, admitted on a cue word |
| 0.95 | No check digit; structure plus authoritative list membership (email TLD, BIC country, MMSI MID) |
| 0.90 | No check digit; structure plus a cue word |
| 0.85 | Structure and a plausibility window only |
| 0.80 | Structure only, with an ambiguity reported as a flag |

A rule may not claim a number its checks do not earn, and its catalogue entry's `checks` list is the
audit trail for the one it claims. The ladder in full, with the reasoning, is in
[../../docs/Rules_And_Patterns.md](../../docs/Rules_And_Patterns.md).

## At most six rules per entity type

An entity type is a facet somebody indexes, and a facet fed by a dozen loosely related rules stops
meaning one thing. Six is the budget a proposed rule is measured against: when a type is full, the
question is which of its rules is the weakest, not how to raise the cap.
