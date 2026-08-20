//! Cue words: what admits a bare token into an index.
//!
//! Several formats are digit runs with a check digit and nothing else — IMO, MMSI, IMEI, CUSIP,
//! SEDOL. A check digit is a one-in-ten filter, which is not enough on its own to put a nine-digit
//! run into a facet: invoice numbers, part numbers and order references all pass it at that rate.
//! What makes the match defensible is the word beside it, so the rules for those formats require
//! one.
//!
//! The cue list belongs to the rule and appears verbatim in its catalogue entry, because "this was
//! accepted because the word IMO was nearby" is exactly what a reader needs in order to weigh the
//! match.

use crate::rules::Candidate;

/// How far either side of a candidate a cue word counts. Roughly a clause: far enough to catch
/// `IMO number 9319466` and a label in a table cell, short enough that a cue two sentences away
/// does not vouch for an unrelated number.
pub const DEFAULT_CUE_WINDOW: usize = 48;

impl Candidate<'_> {
    /// Whether one of `cues` appears within `window` bytes before or after the candidate, matched
    /// case-insensitively and on ASCII word boundaries.
    ///
    /// The boundaries are what keep `imo` from matching inside `Timor` or `imovable`, which is the
    /// difference between a guard and a coincidence.
    pub fn has_cue(&self, cues: &[&str], window: usize) -> bool {
        let before = self.slice_before(window).to_ascii_lowercase();
        let after = self.slice_after(window).to_ascii_lowercase();
        cues.iter().any(|cue| {
            let cue = cue.to_ascii_lowercase();
            !cue.is_empty() && (contains_word(&before, &cue) || contains_word(&after, &cue))
        })
    }

    /// Up to `window` bytes of the fragment ending where the candidate starts, trimmed back to a
    /// character boundary.
    fn slice_before(&self, window: usize) -> &str {
        let mut from = self.start.saturating_sub(window);
        while from < self.start && !self.fragment.is_char_boundary(from) {
            from += 1;
        }
        &self.fragment[from..self.start]
    }

    /// Up to `window` bytes of the fragment starting where the candidate ends, trimmed forward to
    /// a character boundary.
    fn slice_after(&self, window: usize) -> &str {
        let mut to = self.end.saturating_add(window).min(self.fragment.len());
        while to > self.end && !self.fragment.is_char_boundary(to) {
            to -= 1;
        }
        &self.fragment[self.end..to]
    }
}

/// `needle` inside `haystack` with an ASCII non-alphanumeric on both sides, or the edge of the
/// slice. Both arguments are already lowercased.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    haystack.match_indices(needle).any(|(at, _)| {
        let left_clear = at == 0 || !is_word_byte(bytes[at - 1]);
        let end = at + needle.len();
        let right_clear = end == bytes.len() || !is_word_byte(bytes[end]);
        left_clear && right_clear
    })
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use crate::data::VendoredData;
    use crate::rules::context::DEFAULT_CUE_WINDOW;
    use crate::rules::Candidate;

    fn candidate<'a>(fragment: &'a str, data: &'a VendoredData, needle: &str) -> Candidate<'a> {
        let start = fragment
            .find(needle)
            .expect("the needle is in the fragment");
        Candidate {
            fragment,
            start,
            end: start + needle.len(),
            data,
        }
    }

    #[test]
    fn a_cue_counts_on_either_side_and_ignores_case() {
        let data = VendoredData::default();
        for fragment in [
            "IMO 9319466 arrived",
            "9319466 (imo) arrived",
            "vessel 9319466, IMO number, arrived",
        ] {
            assert!(
                candidate(fragment, &data, "9319466").has_cue(&["imo"], DEFAULT_CUE_WINDOW),
                "{fragment:?}"
            );
        }
    }

    /// A cue that is only a substring of a longer word vouches for nothing, and one past the window
    /// is not in reach.
    #[test]
    fn a_cue_needs_word_boundaries_and_has_to_be_in_reach() {
        let data = VendoredData::default();
        assert!(!candidate("Timor 9319466 arrived", &data, "9319466")
            .has_cue(&["imo"], DEFAULT_CUE_WINDOW));
        assert!(!candidate(
            "imo, and then a long stretch of unrelated prose, 9319466",
            &data,
            "9319466"
        )
        .has_cue(&["imo"], DEFAULT_CUE_WINDOW));
    }
}
