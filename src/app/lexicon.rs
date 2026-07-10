//! The naming-ceremony lexicon: a bank of Culture-style ship names (in the
//! spirit of Iain M. Banks — wry, understated noun phrases), used to pre-fill
//! the rename wizards with a suggestion the pilot can keep, edit, or clear.
//!
//! These are original coinages in that register, not Banks's own names.

/// The suggestion bank. Kept under 40 chars each (the Manny name limit).
pub const NAMES: &[&str] = &[
    "Statistical Outlier",
    "Polite Refusal",
    "Considered Opinion",
    "Reasonable Doubt",
    "Tactical Withdrawal",
    "Calculated Risk",
    "Second Thoughts",
    "Margin Of Error",
    "Amiable Disagreement",
    "Sudden Enthusiasm",
    "Prudent Distance",
    "Diminishing Returns",
    "Casual Observer",
    "Selective Memory",
    "Unlikely Story",
    "Graceful Degradation",
    "Nominal Trajectory",
    "Cautious Optimism",
    "Mild Curiosity",
    "Provisional Conclusion",
    "Rounding Error",
    "Fashionably Late",
    "No Particular Hurry",
    "Against Better Judgement",
    "Some Assembly Required",
    "Ends Well Enough",
];

/// The suggestion at index `n` (wrapping), so a monotonically-increasing seed
/// cycles through the bank.
pub fn suggest(n: usize) -> &'static str {
    NAMES[n % NAMES.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_wraps_and_names_fit_the_manny_limit() {
        assert!(!NAMES.is_empty());
        assert_eq!(suggest(0), NAMES[0]);
        assert_eq!(suggest(NAMES.len()), NAMES[0], "index wraps");
        assert!(
            NAMES.iter().all(|n| n.chars().count() <= 40),
            "names stay within the 1-40 char Manny limit",
        );
    }
}
