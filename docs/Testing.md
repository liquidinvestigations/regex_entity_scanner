# Testing

```sh
./test.sh              # everything: rustfmt, clippy, the test battery
./test.sh email        # filter, passed through to cargo test
./test-long.sh         # the upstream conformance run, separately
./test-long.sh stdnum  # one origin, named by its case file's stem
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

The harness scores precision and recall over the whole corpus; `./test.sh -- --nocapture` shows the
numbers on a passing run, and a failure lists every spurious and missed span. Roughly half the
cases expect nothing at all: precision is the property under test, and a corpus of positives alone
cannot measure it.

Fixtures name the surface form and the canonical value rather than byte offsets, because offsets
counted by hand are wrong often enough to stop people extending the corpus. The harness separately
asserts that every reported span really covers the text it claims, which is the property those
offsets were needed for.

**Every rule change adds a case**: the input that motivated it, and the near-miss that must stay
rejected. A rule change with no new fixture is a change nobody can defend later.

## The upstream conformance run

The golden corpus measures us against cases we wrote. `./test-long.sh` measures us against cases the
upstream implementers wrote. Seven origins carry it: `python-stdnum`, whose every module docstring is
a labelled valid/invalid corpus for its own scheme; `libphonenumber`, whose metadata holds an
example number for every region and line type and whose matcher test holds a free-text corpus;
`recognizers-text`, whose specs are sentences with the extractions expected from them;
`price-parser`, whose test data is several hundred price strings from real pages; `grok`, whose
specs pair each log pattern with the log lines it must match, which is where machine date formats
live; and `dateparser`, whose parser tests are a parametrised list of date strings with the
datetime each parses to, and two tests that declare a string unparseable; and `presidio`, whose
recogniser tests are free-text fragments with the spans upstream asserts are in them, including the
fragments it asserts hold nothing. Agreement with
upstream is the only evidence that a ported check digit, or a ported separator rule, means what its
author meant.

It is a separate entry point with its own budget of fifteen to twenty minutes, and the fast battery
never triggers it — the two-to-three-minute budget above is not something a conformance sweep may
spend. The runner samples each scheme deterministically rather than exceeding its budget, taking the
first cases of a scheme in the case file's sorted order, so the sample is the same on every machine
with no seed to carry.

The run asserts a recall and a precision floor on the aggregate **and on every origin
separately**, in `tests/conformance.rs`. The per-origin floors are what stop one origin regressing
behind the others: the largest origin carries twenty times the cases of the smallest, so a rule that
loses every case in a small one moves the aggregate by less than the aggregate floor's own margin.
Every origin in the corpus must have a floor, so adding an origin is a decision about the number it
is expected to hold. The floors are a ratchet — raised after a real fix, never lowered to make a run
pass.

Three numbers are reported per scheme, per origin and in aggregate, and never blended: **recall**
over upstream-valid cases in a scheme we implement, **precision** over upstream-invalid ones — did
we stay silent — and **coverage**, how much of the extracted material was scored at all. Excluded
cases are counted and listed with their reason beside every score, because a run that excludes its
way to a good number measures nothing. `tests/conformance/Readme.md` has the case-file schema, the
carrier and cue rules, and what may be excluded.

## One scanner per test binary

`tests/support::scanner()` hands out a shared `Arc<Scanner>` from a `OnceLock`. Compiling every
candidate pattern and loading the vendored data files is the expensive part of a test, and paying
that once per test instead of once per binary is where the whole budget would go. Nothing in the
scanner is mutable, so sharing it across tests changes no result.

## Where identifier fixtures come from

Every `python-stdnum` module under `vendored/reference/` carries a valid and an invalid example in
its docstring, and those are the fixtures: numbers the upstream implementers chose as the ones worth
testing. Roughly five valid and three invalid per scheme, and the test says what the subset covers —
the formats and the failure modes that scheme claims. Inventing an identifier by hand mostly proves
the check digit routine agrees with itself.

## What the other tests cover

Per-rule files pin what the validator keeps, what it rejects, and what the value normalises to —
including the rejections that matter most, since a false positive is the failure mode with real
cost. `http.rs` exercises the service over a real socket on an ephemeral port, so it never collides
with a server already running on the published one.

Tests build their scanner over the vendored tree next to the manifest rather than through
`RES_VENDORED_DIR`, so they do not depend on the environment the way the binary does. They will fail
with a clear message if the vendored data has not been fetched.
