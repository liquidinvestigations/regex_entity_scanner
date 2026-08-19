# Testing

```sh
./test.sh              # everything: rustfmt, clippy, the test battery
./test.sh email        # filter, passed through to cargo test
```

`test.sh` builds the dev image if it is missing, then runs `cargo fmt --check`, `cargo clippy` with
warnings denied, and `cargo test`.

## The speed budget is a design constraint

The whole battery stays inside **two to three minutes**, and an individual test inside **ten to
fifteen seconds**. This is not a nicety. The only way precision work gets done at all is if the loop
is short enough to run on every rule change, and a suite that is slow enough to skip is a suite
nobody runs.

Reference corpora and upstream test suites are large, and running one whole is the usual way this
budget gets lost. A test iterates a **representative subset** — chosen to cover the formats, locales
and failure modes that rule actually claims — and says in the test what the subset is for. That is
thoroughness; iterating everything is just slowness that looks like thoroughness.

## The golden corpus

`tests/golden/corpus.jsonl` is the test that decides whether a rule change was an improvement. Unit
tests pin individual behaviours; only a labelled corpus answers "did this pattern start swallowing
version numbers", which is the way this kind of system actually fails — not by being slow, but by
filling a facet with noise until nobody uses it.

The harness scores precision and recall over the whole corpus and prints both. Roughly half the
cases expect nothing at all: precision is the property under test, and a corpus of positives alone
cannot measure it.

Fixtures name the surface form and the canonical value rather than byte offsets, because offsets
counted by hand are wrong often enough to stop people extending the corpus. The harness separately
asserts that every reported span really covers the text it claims, which is the property those
offsets were needed for.

**Every rule change adds a case**: the input that motivated it, and the near-miss that must stay
rejected. A rule change with no new fixture is a change nobody can defend later.

## What the other tests cover

Per-rule files pin what the validator keeps, what it rejects, and what the value normalises to —
including the rejections that matter most, since a false positive is the failure mode with real
cost. `http.rs` exercises the service over a real socket on an ephemeral port, so it never collides
with a server already running on the published one.

Tests build their scanner over the vendored tree next to the manifest rather than through
`RES_VENDORED_DIR`, so they do not depend on the environment the way the binary does. They will fail
with a clear message if the vendored data has not been fetched.
