use std::collections::HashSet;

pub const CATEGORIES: [&str; 7] = [
    "security",
    "industry-policy",
    "development-infrastructure",
    "ai",
    "engineering-practice",
    "research-lectures",
    "software-products",
];

#[derive(Debug, PartialEq, Eq)]
pub enum RuleError {
    Count { category: String, actual: usize },
    DuplicateEvent(String),
    UnknownCategory(String),
    InvalidWindow,
    OutsideGenerationWindow,
    UnobservedEvidence,
    MissingContinuity,
    UnsupportedClaim,
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RuleError {}

pub fn entry_counts<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<(), RuleError> {
    let mut counts = [0; 7];
    let mut events = HashSet::new();
    for (category, event) in entries {
        let index = CATEGORIES
            .iter()
            .position(|c| *c == category)
            .ok_or_else(|| RuleError::UnknownCategory(category.into()))?;
        if !events.insert(event) {
            return Err(RuleError::DuplicateEvent(event.into()));
        }
        counts[index] += 1;
    }
    for (category, count) in CATEGORIES.iter().zip(counts) {
        if !(4..=7).contains(&count) {
            return Err(RuleError::Count {
                category: (*category).into(),
                actual: count,
            });
        }
    }
    Ok(())
}

pub fn generation_window(
    previous: i64,
    cutoff: i64,
    deadline: i64,
    now: i64,
) -> Result<(), RuleError> {
    if !(23 * 3600..=25 * 3600).contains(&(cutoff - previous)) || deadline - cutoff != 3600 {
        return Err(RuleError::InvalidWindow);
    }
    if now < cutoff || now >= deadline {
        return Err(RuleError::OutsideGenerationWindow);
    }
    Ok(())
}

pub fn evidence_at_cutoff(first_seen: i64, retrieved: i64, cutoff: i64) -> Result<(), RuleError> {
    if first_seen > cutoff || retrieved > cutoff {
        return Err(RuleError::UnobservedEvidence);
    }
    Ok(())
}

pub fn continuity(
    previous: Option<&str>,
    fingerprint: &str,
    label: &str,
    delta: &str,
) -> Result<(), RuleError> {
    match (previous, label) {
        (Some(old), "update") if old != fingerprint && !delta.trim().is_empty() => Ok(()),
        // Changed source text is a signal for model reassessment, not proof of a material update.
        (Some(_), "carryover") if delta.trim().is_empty() => Ok(()),
        (None, "new" | "late-discovery" | "older-work") => Ok(()),
        _ => Err(RuleError::MissingContinuity),
    }
}

pub fn quote_supported(document: &str, quote: &str) -> Result<(), RuleError> {
    if quote.trim().len() < 24 || !document.contains(quote) {
        return Err(RuleError::UnsupportedClaim);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(n: usize) -> Vec<(String, String)> {
        CATEGORIES
            .iter()
            .flat_map(|c| (0..n).map(move |i| (c.to_string(), format!("{c}-{i}"))))
            .collect()
    }
    fn check(rows: &[(String, String)]) -> Result<(), RuleError> {
        entry_counts(rows.iter().map(|(a, b)| (a.as_str(), b.as_str())))
    }
    #[test]
    fn all_seven_categories_require_four_to_seven() {
        for n in 0..=8 {
            assert_eq!(check(&entries(n)).is_ok(), (4..=7).contains(&n));
        }
    }
    #[test]
    fn duplicate_across_columns_rejected() {
        let mut rows = entries(4);
        rows[5].1 = rows[0].1.clone();
        assert!(matches!(check(&rows), Err(RuleError::DuplicateEvent(_))));
    }
    #[test]
    fn unknown_category_rejected() {
        assert!(matches!(
            entry_counts([("news", "x")]),
            Err(RuleError::UnknownCategory(_))
        ));
    }
    #[test]
    fn daylight_saving_windows_and_deadline() {
        for hours in [23, 24, 25] {
            let cutoff = hours * 3600;
            assert!(generation_window(0, cutoff, cutoff + 3600, cutoff).is_ok());
            assert!(generation_window(0, cutoff, cutoff + 3600, cutoff + 3599).is_ok());
            assert!(generation_window(0, cutoff, cutoff + 3600, cutoff - 1).is_err());
            assert!(generation_window(0, cutoff, cutoff + 3600, cutoff + 3600).is_err());
        }
        assert!(generation_window(0, 22 * 3600, 23 * 3600, 22 * 3600).is_err());
    }
    #[test]
    fn post_cutoff_evidence_rejected() {
        assert!(evidence_at_cutoff(10, 20, 20).is_ok());
        assert!(evidence_at_cutoff(21, 20, 20).is_err());
        assert!(evidence_at_cutoff(10, 21, 20).is_err());
    }
    #[test]
    fn updates_need_changed_evidence_and_specific_delta() {
        assert!(continuity(Some("a"), "b", "update", "Mitigation now available").is_ok());
        assert!(continuity(Some("a"), "a", "update", "Same story").is_err());
        assert!(continuity(Some("a"), "b", "update", "").is_err());
        assert!(continuity(Some("a"), "a", "new", "").is_err());
        assert!(continuity(Some("a"), "a", "carryover", "").is_ok());
        assert!(continuity(Some("a"), "b", "carryover", "").is_ok());
        assert!(continuity(Some("a"), "b", "carryover", "New mitigation").is_err());
        assert!(continuity(None, "a", "late-discovery", "").is_ok());
    }
    #[test]
    fn quotes_must_be_substantial_and_exact() {
        let text = "Release 1.2.3 fixes the connection pool deadlock.";
        assert!(quote_supported(text, text).is_ok());
        assert!(
            quote_supported(text, "Release 1.2.4 fixes the connection pool deadlock.").is_err()
        );
        assert!(quote_supported(text, "1.2.3").is_err());
    }
}
