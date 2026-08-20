# `src/rules/`

One module per entity type. A rule is a candidate pattern plus a validator, and adding a type is a
pattern, a validator and a fixture — not a new pipeline.

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
4. **List membership** — the IANA TLD list, airport codes, MMSI flag prefixes. Cheap, and decisive
   for the types that have no checksum.
5. **Ambiguous-lexeme rules** — surface forms that collide with ordinary words and need stronger
   context before they count.

## Confidence

Confidence is what a consumer thresholds on at index time, so it has to mean something consistent
across rules: how likely this span is to be the thing the rule claims, given everything the
validator checked. A rule that passed a checksum says so with a high number; a rule that only
matched a shape and a cue word does not get to claim the same.
