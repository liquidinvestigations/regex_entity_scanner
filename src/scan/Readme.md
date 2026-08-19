# `src/scan/`

The pipeline.

```text
fragment ─► prefilter ─► validate ─► normalise ─► resolve ─► entities
```

`validate` and `normalise` live in the rules rather than here, because both are per-type knowledge
and splitting them would only mean passing a half-checked value between two functions that always
run together.

## `prefilter.rs`

Two passes, on purpose. The `RegexSet` pass answers "does any rule fire anywhere in this fragment"
for the whole rule set in one go, and most fragments in a real corpus contain none of what any given
rule is looking for. Only the rules that survive that answer then scan for their own spans. At a
handful of rules this is a small win; at several hundred it is the difference between one pass over
the text and several hundred.

Candidates are collected per rule, so two rules may propose spans that overlap or nest. That is
intended, and it is `resolve.rs`'s job to decide between them.

The engine stays behind this module deliberately. Multi-pattern matching and literal prefiltering
are where the large factors live; a faster engine underneath is an optimisation this boundary allows
without touching a single rule.

## `resolve.rs`

Structural strength first, then length, then position.

Strength before length because a match that had to satisfy a checksum or a calendar is the less
accidental reading of the two, even when it is the shorter one. Length after that because within one
type the longer match is the more completely parsed one — a full timestamp beats the bare date
inside it. Position last, only to make the result deterministic.

Emitting both readings instead would put one document in two facets on the strength of one number,
which is exactly how a facet stops being trustworthy.
