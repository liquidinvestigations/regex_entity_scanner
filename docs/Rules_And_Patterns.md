# Rules and patterns

A rule is a candidate pattern plus a validator. Adding an entity type is a pattern, a validator and
a fixture — not a new pipeline.

## What each half may do

**The candidate pattern** is compiled into the prefilter alongside every other rule's and runs over
raw document text. It must be expressible in a linear-time engine: no lookaround, no
backreferences, and no `^`, `$` or `\b` — see below. It is expected to over-match — being generous
here is free, because everything it proposes is checked.

**The validator** sees one candidate at a time, with the surrounding fragment available, and may be
as expensive as it likes. It returns nothing to reject the candidate, or a verdict carrying the
canonical value, a confidence, any flags, and possibly a narrowed span.

## Stripped guards

Upstream pattern corpora lean on lookaround, and the best of them — Microsoft's Recognizers-Text —
is .NET flavour throughout. Those guards do not survive the port, and they should not: a lookbehind
`(?<!\d)` is exactly "the byte before this span is not a digit", which is one comparison in the
validator once a candidate exists. Strip the guard from the pattern, re-impose it in the validator,
and the scanning pass stays linear.

The same move handles trailing punctuation. A pattern that tries to exclude the full stop at the end
of a sentence becomes unreadable; a validator that trims a trailing `.` is one line.

## No anchors, no `\b`

A candidate pattern must not depend on where its haystack begins or ends, because it is re-run over
a **slice** of the fragment whenever the scan loop looks inside a rejected candidate. `^`, `$` and
`\b` all mean something different against a slice than against the whole fragment, and the
difference is silent. The prefilter refuses to compile a pattern containing any of them at startup,
so the constraint is enforced rather than remembered.

Nothing is lost by it. A boundary condition is validator work and always was: the validator holds
the whole fragment and absolute offsets, so "the preceding byte is not a digit" is one comparison,
and it is the same move that strips a lookbehind out of the pattern.

## Looking inside a rejected candidate

The prefilter takes leftmost-longest per rule, so a long over-matched candidate that fails its
validator routinely contains a shorter one that would pass — an identifier-shaped run whose tail is
a page number, a timestamp with an impossible clock wrapped around a perfectly good date, an address
ending in a top-level domain that does not exist. If rejection were terminal all of those would be
lost.

So a rejection shrinks the candidate by one character from the right, re-runs the rule's own pattern
over the interior, and queues whatever it finds. That yields both cases in one mechanism: the
shorter match at the same start, and the nested match that starts later. The validator is unchanged
— it still receives the whole fragment and absolute offsets, so the adjacent-byte guards read the
real neighbours rather than the edges of a slice.

**The recovery is bounded and therefore best-effort: eight retries per candidate, two hundred and
fifty-six per fragment.** A valid match nested more deeply than that is still lost, and that is the
right trade: unbounded retry is quadratic in the fragment on adversarial input, which is exactly the
property the linear-time prefilter exists to protect.

One consequence for new rules: a shortened re-scan is a search for a shorter match, and short
strings are far more likely to be accidentally valid — a great many two-letter sequences are
registered top-level domains. A rule whose format has a variable length needs its right-hand
boundary guard as much as its left.

## Guards that earn their keep

Roughly in order of noise removed per line of code:

1. **Structural validity.** A calendar that has the day; a check digit that agrees. This alone
   removes the bulk of numeric false positives, and for the identifier types it is close to
   decisive — a check digit is a one-in-ten to one-in-ninety-seven filter applied for free.
2. **Plausibility windows.** A year range, an amount magnitude, a unit magnitude. Kills version
   strings and identifier fragments.
3. **Adjacent-byte guards.** The stripped lookarounds. A date wedged between digits is part of a
   longer number.
4. **List membership.** The IANA TLD list, airport codes, MMSI flag prefixes. Cheap, and decisive
   for the types that have no checksum — though for short tokens membership is necessary and never
   sufficient, and those rules also demand a cue word nearby.
5. **Ambiguous-lexeme rules.** Surface forms that collide with ordinary words — bare `m`, `in`,
   `t`, `$` — need stronger context before they count at all.
6. **Negative contexts.** Inside a URL, inside a base64 blob, inside a hash, inside a code block.

## Confidence

Confidence is thresholded by consumers at index time, so it must mean the same thing across rules:
how likely this span is to be what the rule claims, given what the validator could actually check. A
rule that verified a check digit says so with a high number. A rule that matched a shape and found a
cue word nearby does not get to claim the same, however useful its output is.

## Rule identifiers

Dotted, general to specific: `date.iso8601`, `email.basic`. The identifier ships in the output and
lands in the index, which makes renaming one a data migration rather than a tidy-up.

## Every rule is documented

A rule is not finished when it matches correctly. It is finished when a reader who clicks one of its
matches can find out what it is: a catalogue entry in `src/explain/catalog.rs` naming what it
matches, the standards behind it, what the validator checks, what acceptance does **not** prove, the
authorities and the references. The test suite asserts that every compiled rule has one. See
[Explainer_Cards.md](Explainer_Cards.md).

## What does not belong here

Person names, organisation names, locations, job titles, relationships, events. The division is
clean and worth holding: **this layer owns everything with machine-checkable structure; an NLP layer
owns everything defined by meaning.** They meet at entity resolution — a validated identifier is the
anchor that disambiguates a fuzzy name match, which is the pairing that makes an index useful.
