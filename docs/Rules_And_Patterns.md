# Rules and patterns

A rule is a candidate pattern plus a validator. Adding an entity type is a pattern, a validator and
a fixture — not a new pipeline.

## What each half may do

**The candidate pattern** is compiled into the prefilter alongside every other rule's and runs over
raw document text. It must be expressible in a linear-time engine: no lookaround, no
backreferences. It is expected to over-match — being generous here is free, because everything it
proposes is checked.

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

## What does not belong here

Person names, organisation names, locations, job titles, relationships, events. The division is
clean and worth holding: **this layer owns everything with machine-checkable structure; an NLP layer
owns everything defined by meaning.** They meet at entity resolution — a validated identifier is the
anchor that disambiguates a fuzzy name match, which is the pairing that makes an index useful.
