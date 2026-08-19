# `tests/`

| File | What it pins |
|---|---|
| `support/mod.rs` | Builds a scanner over the vendored tree next to the manifest, so tests do not depend on the environment the way the binary does. |
| `dates.rs`, `email.rs` | Per-rule behaviour: what the validator keeps, what it rejects, and what the value normalises to. |
| `golden.rs` | The labelled corpus, scored as precision and recall. |
| `http.rs` | The service surface over a real socket. |

## The golden corpus

`golden/corpus.jsonl` is the test that decides whether a rule change was an improvement. Unit tests
pin individual behaviours; only a labelled corpus answers "did this pattern start swallowing version
numbers", which is how this kind of system actually fails.

Roughly half the corpus is fragments where the right answer is nothing at all. Negative cases are
not padding — precision is the property under test, and a corpus of positives alone cannot measure
it.

Fixtures name the surface form and the canonical value rather than byte offsets, because offsets
counted by hand are wrong often enough to stop people extending the corpus. The harness separately
asserts that every reported span really does cover the text it claims, which is the property the
offsets needed to prove.

Add a case for every rule change: the input that motivated it, and the near-miss that must stay
rejected.

## Speed

The whole battery stays inside two to three minutes and an individual test inside ten to fifteen
seconds. Reference corpora are large; a test iterates a representative subset and says what the
subset covers. A suite that takes long enough to skip is not more thorough, it is unused.
