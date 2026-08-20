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

`compile` refuses a candidate pattern containing `^`, `$` or `\b`. Patterns are re-run over slices
of a fragment by the scan loop below, and an anchor or a word boundary means something different
against a slice than against the whole text — silently. Boundary conditions are validator work.

The engine stays behind this module deliberately. Multi-pattern matching and literal prefiltering
are where the large factors live; a faster engine underneath is an optimisation this boundary allows
without touching a single rule.

## The work list in `mod.rs`

Scanning is a work list, not a loop over the prefilter's output. Each item is a rule index and a
span; a visited set on `(rule, start, end)` keeps the same candidate from being validated twice.

A rejected candidate is queued back, shrunk. The prefilter takes leftmost-longest per rule, so a
long over-matched span that fails its validator routinely contains a shorter one that would pass —
an identifier-shaped run whose tail is a page number, an impossible clock wrapped around a good
date, an address ending in a top-level domain that does not exist. Rejection therefore shrinks the
candidate by one character from the right and re-runs the rule's own pattern over the interior,
which finds both the shorter match at the same start and the nested match that starts later.

The validator sees the whole fragment and absolute offsets on a retry exactly as on a first pass, so
the adjacent-byte guards read the real neighbours rather than the edges of a slice.

The recovery searches the rejected candidate's **interior**, which is what bounds its cost, and that
bound has a visible edge: a match that begins inside a rejected candidate and ends past its right
edge is not recovered. In `1 $10 dollars bill` a money pattern's trailing-sign alternative matches
`1 $` first — the engine is leftmost-first, so a match at offset 0 wins over the better one at
offset 2 — the validator rejects it because a digit follows, and `$10` is never proposed at all.
Recovering it means re-searching the rest of the fragment on every rejection, which is linear work
per rejection and is exactly the cost the caps exist to refuse.

The retry is capped at eight per candidate and two hundred and fifty-six per fragment, which makes
it best-effort by construction. That is deliberate: unbounded retry is quadratic in the fragment on
adversarial input, which is the property the linear-time prefilter exists to protect. A match nested
more deeply than the caps allow is lost, and losing it is cheaper than the alternative.

## `resolve.rs`

Structural strength first, then length, then position.

Strength before length because a match that had to satisfy a checksum or a calendar is the less
accidental reading of the two, even when it is the shorter one. Length after that because within one
type the longer match is the more completely parsed one — a full timestamp beats the bare date
inside it. Position last, only to make the result deterministic.

Emitting both readings instead would put one document in two facets on the strength of one number,
which is exactly how a facet stops being trustworthy.

Because the loser is dropped, **a bad candidate here can delete a correct entity rather than merely
add a wrong one, which makes one false positive twice as expensive as it looks.** The shape is an
angle-bracketed address in a mail header: read as a message id it outranks the address reading and
is longer, so the address never reaches the output and the fragment loses the only entity it
actually contained. The answer is not a different ladder — on a genuine `Message-ID:` line the
message-id reading is the right one and the address reading has to lose — but a rule that does not
propose the reading in the first place.
