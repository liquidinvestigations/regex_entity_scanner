# `tests/conformance/`

The upstream conformance corpus: cases taken from the test material of the projects our rules were
ported from, so a run answers "do we agree with the people who implement this scheme" rather than
"do we agree with ourselves". `./test-long.sh` runs it; `tests/conformance.rs` is the runner and
scorer.

| File | What it holds |
|---|---|
| `extract_stdnum.py` | Turns the `python-stdnum` module docstrings into cases. Runs in the dev container. |
| `stdnum.jsonl` | The extracted `python-stdnum` cases, checked in. |
| `extract_libphonenumber.py` | Turns libphonenumber's per-region example numbers and its matcher's free-text corpus into cases. |
| `libphonenumber.jsonl` | The extracted libphonenumber cases, checked in. |
| `extract_recognizers.py` | Turns the Recognizers-Text English specs — dates, currency, IP, phone, email — into cases. |
| `recognizers.jsonl` | The extracted Recognizers-Text cases, checked in. |
| `extract_price_parser.py` | Turns price-parser's parametrised (text, currency, amount) rows into cases. |
| `price-parser.jsonl` | The extracted price-parser cases, checked in. |
| `extract_grok.py` | Turns the logstash-patterns-core specs — the log lines each grok pattern must match — into date cases. |
| `grok.jsonl` | The extracted logstash-patterns-core cases, checked in. |
| `extract_dateparser.py` | Turns dateparser's parametrised date strings, and the two tests that declare a string unparseable, into date cases. |
| `dateparser.jsonl` | The extracted dateparser cases, checked in. |
| `extract_presidio.py` | Turns Presidio's recogniser tests — parametrised free-text fragments with the spans upstream asserts are in them — into cases. |
| `presidio.jsonl` | The extracted Presidio cases, checked in. |

## The case file

One JSON object per line, sorted by `id`, with the same shape for every origin. The file is checked
in, so a run is reproducible without re-extracting — extraction is a separate step, taken when the
upstream snapshot moves.

| Field | Meaning |
|---|---|
| `id` | Stable, unique, and prefixed with the scheme, so sorting groups a scheme's cases together. |
| `origin` | The upstream project the case came from. |
| `source` | The upstream file, so a disagreement can be read in context. |
| `scheme` | The upstream scheme or recogniser. This is the unit the report tables are grouped by. |
| `token` | The upstream token, verbatim. |
| `text` | The fragment we scan: the token inside a carrier sentence. |
| `token_start`, `token_end` | Byte offsets of the token within `text`. |
| `cue` | The cue word placed in the carrier, or `null` when the rule requires none. |
| `valid` | Whether upstream considers the token valid. |
| `rule_id`, `entity_type` | What we should produce, or `null` when we implement nothing for the scheme. |
| `expect_value` | The normalised value we should produce, or `null` when upstream documents none. |
| `exclusion` | Why the case is counted but not scored, or `null`. |

An `expect_value` is compared as a string, with one exception: a geographic point is a
comma-separated pair of doubles and is compared componentwise within a billionth of a degree. Two
implementations of the same code arrive at a coordinate by different arithmetic and disagree in the
last bits of the double; that is a fact about floating point and not a disagreement about the place,
and a billionth of a degree is a tenth of a millimetre. The tolerance is one constant in the runner
rather than a per-case field, so no extractor has to fill in a column that one origin needs.

The per-scheme table carries an `unchk` column: agreements where upstream documents no canonical
form, so only the entity type was checked. They count towards recall like any other agreement, and
the column is what keeps a recall figure readable as the mix of evidence it is.

## Turning an upstream test into a scan

Upstream calls an API on a bare token; we scan free text. Three rules bridge that, applied
uniformly and recorded in the file rather than reinvented by the runner:

- **The carrier**, for an origin that tests an API on a bare token. Every token goes into one neutral sentence — `The record shows <token> on the
  form.` — which carries no digits, no currency symbol and no cue word of its own, so the only
  thing the scanner can find in a case is the token.
- **The cue.** Several rules refuse a bare token unless a cue word is nearby, because a check digit
  alone is a one-in-ten filter and an invoice number passes it at that rate. Calling
  `stdnum.cusip.validate` *is* the statement "this is a CUSIP", so that scheme's own documented cue
  goes into the carrier. A cue is never supplied to a rule that does not require one — that would
  be measuring a different rule than the one that runs in production.
- **The token is verbatim.** Separators, case and punctuation are left exactly as upstream wrote
  them. How a number survives contact with real prose is the property under test, so normalising it
  first would delete the finding.

An origin whose own material is already free text keeps that text instead of the carrier, and takes
the offsets upstream reports. Nothing is gained by re-housing a sentence somebody wrote to exercise
a recogniser, and the surrounding words are half of what the case is testing.

A case whose token spans the whole fragment is an upstream input that one recogniser found nothing
in. Only a match of the same entity type fails it: an email corpus saying "no address in this
sentence" makes no claim about the IP address the sentence also contains. Where upstream names a
span, any type over that span still fails the case.

## Exclusions

A case is counted and listed but kept out of the scores when the scheme is one we implement no rule
for, when the upstream call takes arguments our API does not offer, or when the token is in a
format the matching rule's explainer card states it does not match. Every exclusion in the
extractor names the documented limit it rests on, because an exclusion that cannot be traced to the
tracked tree is a thumb on the scale. The run prints the exclusion count beside every score.

## Re-extracting

```sh
./shell.sh python3 tests/conformance/extract_stdnum.py > tests/conformance/stdnum.jsonl
./shell.sh python3 tests/conformance/extract_libphonenumber.py > tests/conformance/libphonenumber.jsonl
./shell.sh python3 tests/conformance/extract_recognizers.py > tests/conformance/recognizers.jsonl
./shell.sh python3 tests/conformance/extract_price_parser.py > tests/conformance/price-parser.jsonl
./shell.sh python3 tests/conformance/extract_grok.py > tests/conformance/grok.jsonl
./shell.sh python3 tests/conformance/extract_dateparser.py > tests/conformance/dateparser.jsonl
./shell.sh python3 tests/conformance/extract_presidio.py > tests/conformance/presidio.jsonl
```

Each script reads its own directory under `vendored/reference/`, needs no network, and writes the
case file to stdout with a count on stderr. Re-run one when the vendored snapshot moves or when a
scheme gains a rule, and commit the diff — the case file is the corpus, not a build artefact.

## National form, and the numbers that are only written in it

libphonenumber's example numbers are *national significant numbers*, and `phone.international`
matches international form only — a deliberate limit, because the same digits name a different
subscriber in every country. Upstream's own test supplies the region as an argument to `parse`;
the extractor supplies the same fact the only way a scanner can receive it, by writing the number
with its country calling code. Scoring a thousand national-form numbers as misses would be
measuring the documented absence of a rule, over and over.
