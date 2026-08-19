# Vendored data

Third-party pattern databases, lexicons and reference data, plus the scripts that fetch them.
Nothing here is ours. Every entry below records where it came from, what it is licensed under, and
what we take from it — a vendored file whose provenance is not written down is trivial to add and
painful to reconstruct later, so adding data without adding its row is not done.

## Layout

| Path | Contents |
|---|---|
| `data/` | What the scanner loads at run time. This is the only part copied into the release image. |
| `reference/` | Upstream material we port from and generate from at development time. Absent from the release image. |
| `licenses/` | Upstream license texts, shipped alongside the data because the licenses require it. |
| `fetch/` | One script per upstream. They run **inside the dev container**. |
| `gleif/data/` | The GLEIF bulk download. Gitignored — it is measured in gigabytes. |

Each vendored directory carries an `.upstream` file naming its source and the commit or version the
snapshot was taken at. That stamp is the one place a date or revision is welcome anywhere in the
tree: it is a fact about the data we ship, and without it the snapshot cannot be diffed against its
upstream.

## Fetching

```sh
./shell.sh vendored/fetch-all.sh              # everything except GLEIF
./shell.sh vendored/fetch-all.sh cldr iana-tlds   # named fetchers only
./shell.sh vendored/fetch/gleif.sh            # gigabytes; only when LEI resolution is wanted
./shell.sh vendored/clone-reference-repos.sh  # full upstreams into tmp/reference, for reading
```

Every fetch stages its download or clone under `tmp/` and copies only the subset we use into place,
so a re-fetch is reviewable as a diff of that subset rather than of an entire upstream tree.

## License compatibility

This project is AGPL-3.0-or-later, which puts us on the receiving end of copyleft — the easy
direction. MIT, BSD-2/3, ISC, Apache-2.0, MPL-2.0, LGPL-2.1+, GPL-3.0, Unicode-3.0, CC0 and
public-domain data all combine into an AGPLv3 work without further obligation beyond keeping the
notices. Two things do not: **GPL-2.0-only**, which is genuinely incompatible, and **Hyperscan 5.5
and later**, which is under the Intel Proprietary License — Vectorscan is the BSD-3 fork of 5.4 and
is the one to use.

Check the licence of a **dataset** separately from the licence of the library that reads it. In this
field they routinely differ, and the datasets are the restrictive half: OurAirports is public
domain and GLEIF is CC0, but OpenFlights is ODbL, OpenSanctions' data is non-commercial-use-only,
OpenSky's aircraft database is effectively unlicensed, and the UK ISCD branch directory is a
commercial feed. Those four are deliberately not vendored here.

## Runtime data — `data/`

| Directory | Upstream | License | What we take, and why |
|---|---|---|---|
| `iana/` | [IANA TLD list](https://data.iana.org/TLD/tlds-alpha-by-domain.txt) | Public domain | The registered top-level domains. One membership test, and most of what an email pattern otherwise matches — file names, sprite paths, internal hostnames — disappears. |
| `ourairports/` | [OurAirports](https://davidmegginson.github.io/ourairports-data/) | Unlicense (public domain) | IATA and ICAO codes for 72k airports, cut to five columns. Membership is necessary but never sufficient: a bare three-letter token collides with ordinary text, so the rule that uses this also requires a cue word. |
| `cldr/` | [unicode-org/cldr-json](https://github.com/unicode-org/cldr-json) | Unicode-3.0 (GPL-compatible) | English names for the ISO 3166-1 alpha-2 territories, flattened to one map. The explainer resolves a country-code top-level domain to the country it belongs to, and a card that says "GB" where it could say "United Kingdom" is a card nobody needed. |
| `itu/` | [michaeljfazio/MIDs](https://github.com/michaeljfazio/MIDs) | MIT | ITU Maritime Identification Digits: the first three digits of an MMSI are its flag state. An MMSI carries no check digit, so a valid MID plus a cue word is the whole precision story for that rule. |

## Development reference — `reference/`

| Directory | Upstream | License | What we take, and why |
|---|---|---|---|
| `recognizers-text/` | [microsoft/Recognizers-Text](https://github.com/microsoft/Recognizers-Text) | MIT | The language-neutral YAML under `Patterns/`: curated, false-positive-tuned patterns and lexicons for dates, currency, units, phones, email and more, in fifteen languages, kept separate from the implementations. **The patterns are .NET flavour and lean on lookaround**, which no linear-time engine accepts — they are input to rule authoring, not something we compile as-is. |
| `cldr/` | [unicode-org/cldr-json](https://github.com/unicode-org/cldr-json) | Unicode-3.0 (GPL-compatible) | `ca-gregorian.json`, `numbers.json` and `units.json` for every modern locale, plus supplemental currency, week and subtag data. The only source that scales past a handful of languages without linear human effort. CLDR describes *formatting*; inverting a format pattern into a parser is our work, and an inverted pattern is always laxer than the formatter that produced it. |
| `dateparser/` | [scrapinghub/dateparser](https://github.com/scrapinghub/dateparser) | BSD-3 | CLDR-derived translation tables for 200+ locales — month and weekday names, separators, skip-words — with a hand-curated supplementary layer over what CLDR misses. Data rather than code, so it ports as a transform. |
| `python-stdnum/` | [arthurdejong/python-stdnum](https://github.com/arthurdejong/python-stdnum) | LGPL-2.1+ | 343 format modules with their real check-digit algorithms: IBAN, BIC, EU VAT, company registration numbers across some sixty jurisdictions, LEI, ISIN, national identity numbers, IMO, MMSI, ISO 6346, IMEI. This is the specification we port from. A check digit turns "matched something plausible" into "verified", which is the difference between a join key and index noise. |
| `libphonenumber/` | [google/libphonenumber](https://github.com/google/libphonenumber) | Apache-2.0 | `PhoneNumberMatcher.java` and `PhoneNumberUtil.java` — the reference implementation of finding numbers in free text, which is the two-stage design in its original form. The metadata itself reaches us through the `phonenumber` crate instead, because regulators reassign number ranges continuously and a frozen copy would rot. |
| `quantulum3/` | [nielstron/quantulum3](https://github.com/nielstron/quantulum3) | MIT | The unit lexicon and entity taxonomy, and the common-words list that keeps bare `m`, `in` and `t` from matching ordinary prose — which is the actual problem with units. |
| `price-parser/` | [scrapinghub/price-parser](https://github.com/scrapinghub/price-parser) | BSD-3 | Separator inference: the best available answer to `1.234` meaning either a thousand or one-and-a-bit depending on who wrote it. |
| `presidio/` | [microsoft/presidio](https://github.com/microsoft/presidio) | MIT | 123 predefined recognisers. Not our engine — Python, no money or units, spans for redaction rather than canonical values — but the best available checklist of PII entity types and per-country identifier formats, each declaring whether it validates by pattern, by checksum or by context. |
| `secret-rules/` | [mongodb/kingfisher](https://github.com/mongodb/kingfisher), [gitleaks/gitleaks](https://github.com/gitleaks/gitleaks) | Apache-2.0, MIT | 623 Kingfisher rule files and the Gitleaks TOML config: the largest maintained bodies of credential and token patterns there are. Kingfisher's schema is also the best model for ours — match, then confirm. |
| `grok/` | [logstash-plugins/logstash-patterns-core](https://github.com/logstash-plugins/logstash-patterns-core) | Apache-2.0 | Battle-tested building blocks for the machine formats that fill logs and mail headers: syslog and common-log timestamps, IPs, URIs, paths, UUIDs, MACs. |
| `followthemoney/` | [alephdata/followthemoney](https://github.com/alephdata/followthemoney) | MIT | The investigative data model, 69 schema files. We take the schema, not the code: naming our output properties the way FtM names them is what makes an extraction usable in the tooling investigative users already run. |

## GLEIF — `gleif/`

[GLEIF open data](https://www.gleif.org/en/about/open-data), CC0. The complete Legal Entity
Identifier population. It is what takes an LEI from a validated string to a resolved company —
legal name, jurisdiction, registered address, status, parent relationships. Fetched on demand by
`fetch/gleif.sh`; the download is gigabytes and is gitignored.
