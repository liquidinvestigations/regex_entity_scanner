# regex-entity-scanner

A stateless Rust service that reads UTF-8 text and returns typed, normalised entity spans: dates
and email addresses today, with money, phone numbers and the large tier of checksummed structured
identifiers built on the same two-stage frame.

It exists because matching is the cheap half of this problem. A pattern that finds every date in a
corpus takes a morning; a pattern that finds every date and nothing that merely looks like one does
not exist. What makes an extraction usable is the work after the match — deciding whether the
candidate is real, and reducing it to a canonical value a range query can use. Everything below
follows from that.

```sh
./build-dev.sh                              # the toolchain image
./shell.sh vendored/fetch-all.sh            # the vendored pattern data
./test.sh                                   # rustfmt, clippy, the battery
./run-server.sh                             # detached on 127.0.0.1:19705
```

```console
$ curl -sS -X POST http://127.0.0.1:19705/scan -H 'content-type: application/json' \
    -d '{"text": "On 2021-03-04T09:12:00+0200 ops@Example.ORG filed it."}' | jq .entities
[
  {
    "type": "date",
    "start": 3, "end": 27,
    "text": "2021-03-04T09:12:00+0200",
    "value": {
      "kind": "date",
      "rfc3339": "2021-03-04T09:12:00+02:00", "precision": "second", "tz_known": true
    },
    "confidence": 0.99,
    "rule_id": "date.iso8601"
  },
  {
    "type": "email",
    "start": 28, "end": 43,
    "text": "ops@Example.ORG",
    "value": {
      "kind": "email",
      "address": "ops@example.org", "local": "ops", "domain": "example.org"
    },
    "confidence": 0.95,
    "rule_id": "email.basic"
  }
]
```

## How it works

```text
fragment ─► prefilter ─► validate ─► normalise ─► resolve ─► entities
```

**Prefilter.** One `RegexSet` pass over the whole rule set says which rules fire anywhere in the
fragment; only those then scan for their own candidate spans. Most fragments contain none of what
any given rule looks for, so that first pass usually eliminates almost every rule. It runs in a
linear-time engine — no lookaround, no backreferences — because the text was written by whoever
wrote the document, and a backtracking engine has no bounded worst case on input like that. It is
expected to over-match.

**Validate.** Per candidate, and allowed to be expensive because it only ever sees candidates. It
re-imposes the guards that had to be stripped out of the pattern — a lookbehind `(?<!\d)` is exactly
"the preceding byte is not a digit", one comparison once a span exists — then applies the checks that
decide truth: calendar validity, check digits, list membership, plausibility windows.

**Normalise.** Span to canonical typed value: an RFC 3339 instant, minor units plus an ISO-4217
code, an SI base unit, E.164, a compact identifier form. This is the half that makes the output
worth indexing.

**Resolve.** Rules scan independently, so their spans can overlap. Structural strength wins first,
then length, then position — a match that had to satisfy a checksum is the less accidental reading
even when it is shorter.

Details in [docs/Architecture.md](docs/Architecture.md).

## HTTP

| Route | Purpose |
|---|---|
| `GET /health` | Liveness, the number of compiled rules, and the size of the loaded TLD list. |
| `GET /rules` | Every documented rule: identifier, type, human-readable title, and whether this build has it compiled. |
| `GET /rules/{rule_id}` | The full static documentation for one rule. |
| `POST /scan` | `{"text": "…", "offset": 0}` → `{"entities": [...], "rule_set_version": 1}`. |
| `POST /explain` | An entity, posted back exactly as `/scan` returned it → an explainer card. |

`offset` is the byte offset of the fragment's first byte in the source document; it is added to
every span, so a caller windowing a large document gets offsets usable against the original bytes.
Windows overlap by at least the longest matchable entity, never split a codepoint, and are
deduplicated on `(type, start, end)`.

Offsets are **bytes**, never characters. `text[start..end]` on the source is always exactly the
reported `text`.

Full contract, including what `confidence`, `rule_id` and the flags mean:
[docs/Entity_Contract.md](docs/Entity_Contract.md).

`rule_set_version` also rides on `/health` and `/rules`. Extraction is not idempotent across rule
sets, so it is what makes the scope of a reindex computable rather than guessed.

Configuration is four environment variables: `RES_BIND` (default `0.0.0.0:19705`),
`RES_VENDORED_DIR`, `RES_MAX_BODY_BYTES` (default `10485760`, applied to both the request body and
the `text` field; an oversized request is a `413`), and `RES_LOG` for a tracing filter. A
`RES_MAX_BODY_BYTES` that does not parse fails startup rather than falling back silently.

## Explainer cards

A reader clicks a highlighted match and wants to know what it is. The client posts the entity back
**unchanged** and gets a card with three text fields — a title, a subtitle and a Markdown body —
plus the same material as structured facts and links.

```console
$ curl -sS -X POST http://127.0.0.1:19705/explain -H 'content-type: application/json' -d @entity.json
{
  "title": "Email address",
  "subtitle": "example.co.uk · United Kingdom",
  "body": "…what the format is, what this value means, what was checked, what it does not prove…",
  "facts": [ {"label": "Country", "value": "United Kingdom"}, … ],
  "references": [ {"title": "IANA register entry for .uk",
                   "url": "https://www.iana.org/domains/root/db/uk.html", "note": "…"}, … ]
}
```

The entity is the whole input — nothing is looked up in a session or in the original document — so a
client can explain a match it stored months ago. Only `rule_id` is required; unknown fields and
unfamiliar value shapes thin the card rather than fail it, because an entity from an older rule set
must still explain itself.

The knowledge is static and lives in `src/explain/catalog.rs`: one entry per rule with what it
matches, the standards that define it, what the validator checks, what acceptance does **not**
prove, the authorities and the references. Per-rule shapers add what only a particular match can
say — the country behind its top-level domain, the weekday its date fell on, the register entry for
its own identifier. Every compiled rule must have an entry, and the test suite fails if one does
not: a reader who clicks a match and gets nothing is worse than a rule that does not exist.

Details in [docs/Explainer_Cards.md](docs/Explainer_Cards.md).

## Entity types

A type is a facet a consumer would build, not a value shape: several types share one `value` shape,
which is why the value carries its own `kind` tag. `date`, `email`, `phone`, `money`,
`bank_account`, `company_id`, `security`, `vessel`, `cargo_container`, `device`, `network`,
`publication`, `vulnerability`, `crypto_wallet`, `coordinates`, `national_id` and `message_id` are
the categories; each holds at most six rules, which is the budget a new rule is measured against.

Compiled today:

| Type | Rule | What the validator checks |
|---|---|---|
| `date` | `date.iso8601` | Real calendar date, real clock time, year inside a plausibility window, no adjacent digits. Normalises to RFC 3339 with an explicit precision, and canonicalises `Z`, `+0200` and `+02:00` to one spelling. |
| `email` | `email.basic` | RFC 5321 length limits, an addressable local part, no adjacent addressable characters on either side, and — the largest single precision win — the top-level domain must be in the IANA list. |

Full RFC 5322 is deliberately not the target for email: it accepts a great deal nobody writes, and
on real corpora the TLD membership test is where the precision actually comes from.

## Containers

Nothing is built or run on the host — no host toolchain, no host Python.

| Script | |
|---|---|
| `build-dev.sh [--force]` | The toolchain image, bind-mounted on the working tree. |
| `build-rel.sh [--force]` | The release image: optimised binary, runtime data, licence notices, nothing else. |
| `shell.sh [command…]` | Throwaway dev container, interactive or running a command. |
| `run-server.sh [--dev]` | Detached server on `127.0.0.1:19705`. |
| `test.sh [filter…]` | rustfmt, clippy, the battery. |

The dev image holds no state: cargo's cache, `target/` and the shell history live under the mount,
so deleting the container costs nothing. The release image carries no toolchain and no sources — it
is around 95 MB and answers on HTTP.

More in [docs/Containers_And_Scripts.md](docs/Containers_And_Scripts.md).

## Vendored data

The patterns and lexicons are not ours and should not be. Month names across two hundred locales,
unit surface forms, currency names, phone metadata, and two hundred check-digit algorithms already
exist upstream under permissive licences; hand-writing any of them would be a mistake rather than
diligence. `vendored/` holds curated subsets of CLDR, Recognizers-Text, dateparser, python-stdnum,
libphonenumber, quantulum3, price-parser, Presidio, Kingfisher, Gitleaks, the grok patterns,
FollowTheMoney, the IANA TLD list, OurAirports and the ITU MID table — each with its upstream,
licence and a fetch script.

```sh
./shell.sh vendored/fetch-all.sh              # everything except the GLEIF bulk download
./shell.sh vendored/clone-reference-repos.sh  # full upstreams into tmp/, for reading
```

Provenance and licences: [vendored/README.md](vendored/README.md).

## Testing

```sh
./test.sh
```

The battery stays inside two to three minutes and each test inside ten to fifteen seconds, because
precision work only happens when the loop is short enough to run on every rule change.

`tests/golden/corpus.jsonl` is the labelled corpus, scored as precision and recall. Roughly half its
cases expect nothing at all — precision is the property under test, and a corpus of positives cannot
measure it. Every rule change adds a case: the input that motivated it, and the near-miss that must
stay rejected.

More in [docs/Testing.md](docs/Testing.md).

## Writing a rule

A candidate pattern, a validator, a fixture, and a catalogue entry so the match can explain itself.
The pattern goes in the prefilter and must stay linear-time; the validator gets one candidate at a
time and does the deciding. Guards that need lookaround move into the validator as byte comparisons.
See [docs/Rules_And_Patterns.md](docs/Rules_And_Patterns.md),
[docs/Explainer_Cards.md](docs/Explainer_Cards.md) and `src/rules/Readme.md`.

Person names, organisations, locations and relationships are deliberately out of scope. This layer
owns everything with machine-checkable structure; meaning belongs to an NLP layer. The two meet at
entity resolution — a validated identifier is the anchor that disambiguates a fuzzy name match.

## Licence

AGPL-3.0-or-later. See [LICENSE](LICENSE), and `vendored/licenses/` for the upstream notices that
travel with the vendored data.
