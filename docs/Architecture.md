# Architecture

## The shape of the problem

Five of the entity types this service is built for — dates, sums of money, physical quantities,
email addresses, phone numbers — plus the much larger tier of structured identifiers, share a
property that decides the whole design: **matching them is the cheap half.**

A pattern that finds every date in a corpus takes a morning. A pattern that finds every date and
nothing that merely looks like one does not exist, because `2021-13-45`, `1.2.3` and the middle of a
SHA-256 hash are all shaped like dates. What separates a usable extraction from index noise is the
work after the match: deciding whether the candidate is real, and reducing it to a canonical typed
value that a range query can use.

So the pipeline is two-stage, and the stages are held to different rules.

```text
fragment (UTF-8, with the byte offset of its first byte in the source document)
        │
        ├─ 1. PREFILTER ── one RegexSet pass over the whole rule set says which rules fire
        │                  anywhere; only those then scan for their own candidate spans.
        │                  Linear-time engine only. Deliberately over-matches.
        │
        ├─ 2. VALIDATE ─── per candidate, allowed to be expensive.
        │                  Re-imposes the guards stripped out of the pattern, checks calendar
        │                  validity, check digits, list membership, plausibility windows.
        │
        ├─ 3. NORMALISE ── span to canonical typed value: RFC 3339 instant, ISO-4217 minor units,
        │                  SI base unit, E.164, compact identifier form.
        │
        ├─ 4. RESOLVE ──── overlaps between rules: structural strength, then length, then position.
        │
        └─ entities, with absolute byte offsets
```

Stages 2 and 3 live together inside each rule, because both are per-type knowledge and separating
them would only mean passing a half-checked value between two functions that always run together.

## Why the prefilter must stay linear-time

The text being scanned is written by whoever wrote the document, which in this domain routinely
means somebody hostile. A backtracking engine has no bounded worst case on input like that, and a
single crafted paragraph is enough to hang a worker. The `regex` crate builds finite automata and is
linear in the input by construction, which is why it is the engine here and why the candidate
patterns carry no lookaround and no backreferences.

That constraint costs something real: the best available upstream pattern corpus is .NET flavour and
leans on lookaround heavily. The answer is not a more permissive engine — it is that a lookbehind
like `(?<!\d)` is exactly the statement "the preceding byte is not a digit", which a validator
checks in one byte comparison once it has a candidate span. The guard moves from the pattern to the
validator and the fast path stays linear.

## Why two passes in the prefilter

`RegexSet::matches` answers "which of these N patterns match anywhere in this text" in one pass.
Most fragments in a real corpus contain none of what any given rule is looking for, so that one pass
usually eliminates almost every rule, and only the survivors scan for spans of their own. At a
handful of rules the saving is small; at several hundred it is the difference between one pass over
the text and several hundred.

## Where the performance actually is

In descending order of effect: multi-pattern single-pass matching, literal prefiltering, not running
expensive validation on non-candidates, and parallelism across fragments. Compiling patterns into
code ahead of time is a real technique and a much smaller lever — a long-lived service compiles its
rule set once at startup and then scans for hours, so compilation cost amortises to nothing.

The engine sits behind `scan::prefilter` for the same reason: swapping in a multi-gigabyte-per-second
prefilter later is a change to one module, and retrofitting that boundary afterwards would not be.
Measure before reaching for it.

## Statelessness

One process, N threads, no models, no writes. The scanner is built once at startup and shared; every
request carries its own fragment. That is what makes the container disposable, horizontally
scalable, and independently versionable from anything else in a pipeline — its rules ship on their
own cadence.
