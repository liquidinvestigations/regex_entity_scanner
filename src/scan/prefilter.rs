//! Stage one: find candidates cheaply, and be wrong in the generous direction.
//!
//! Two passes, for a reason. The `RegexSet` pass answers "does any rule fire anywhere in this
//! fragment" for the whole rule set at once, and most fragments in a real corpus contain none of
//! what any given rule looks for. Only the rules that survive that answer then scan for their own
//! spans. As the rule set grows into the hundreds this is the difference between one pass and
//! hundreds of them, and it is why the candidate patterns must stay expressible in a linear-time
//! engine: they run over raw, attacker-influenceable document text, where a backtracking engine
//! has no bounded worst case.
//!
//! Candidates are collected per rule, so two rules may propose spans that overlap or nest. That is
//! intended — deciding between them is [`super::resolve`]'s job, not this one's.

use anyhow::{Context, Result};
use regex::{Regex, RegexSet};

use crate::rules::Rule;

pub struct Prefilter {
    /// All candidate patterns in one automaton: which rules match anywhere, in a single pass.
    set: RegexSet,
    /// The same patterns individually, for the rules the set says are worth scanning.
    per_rule: Vec<Regex>,
}

impl Prefilter {
    pub fn compile(rules: &[Box<dyn Rule>]) -> Result<Self> {
        let patterns: Vec<&str> = rules.iter().map(|rule| rule.candidate_pattern()).collect();

        let set =
            RegexSet::new(&patterns).context("compiling the combined candidate pattern set")?;

        let per_rule = rules
            .iter()
            .map(|rule| {
                Regex::new(rule.candidate_pattern())
                    .with_context(|| format!("compiling the candidate pattern for {}", rule.id()))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { set, per_rule })
    }

    /// Candidate spans as `(rule index, (start, end))`, in no particular order.
    pub fn candidates(&self, fragment: &str) -> Vec<(usize, (usize, usize))> {
        let mut found = Vec::new();
        for rule_index in self.set.matches(fragment).into_iter() {
            for m in self.per_rule[rule_index].find_iter(fragment) {
                found.push((rule_index, (m.start(), m.end())));
            }
        }
        found
    }
}
