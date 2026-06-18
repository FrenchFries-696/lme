/// Decay score calculation (SRS §2.4).
///
/// decay_score = importance * exp(-lambda * days_since_last_access) * conf_factor
///
/// Uses f64::MIN_POSITIVE floor to prevent underflow (red team F3).
pub fn decay_score(
    importance: u8,
    lambda: f64,
    days_since_access: f64,
    verified: bool,
    conf_inferred: f64,
) -> f64 {
    let conf_factor = if verified { 1.0 } else { conf_inferred };
    let raw = importance as f64 * (-lambda * days_since_access).exp() * conf_factor;
    raw.max(f64::MIN_POSITIVE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_memory_max_score() {
        let score = decay_score(5, 0.005, 0.0, true, 0.6);
        assert!((score - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_old_memory_low_score() {
        let score = decay_score(3, 0.01, 365.0, false, 0.6);
        // 3 * exp(-3.65) * 0.6 ≈ 0.047
        assert!(score < 0.1);
        assert!(score > 0.0);
    }

    #[test]
    fn test_verified_scores_higher() {
        let verified = decay_score(3, 0.01, 100.0, true, 0.6);
        let inferred = decay_score(3, 0.01, 100.0, false, 0.6);
        assert!(verified > inferred);
    }

    #[test]
    fn test_no_underflow() {
        // Very old memory should still be > 0
        let score = decay_score(5, 0.01, 10_000.0, true, 0.6);
        assert!(score > 0.0);
    }

    #[test]
    fn test_arch_slower_than_conversation() {
        let conv = decay_score(3, 0.01, 100.0, true, 0.6);
        let arch = decay_score(3, 0.001, 100.0, true, 0.6);
        assert!(arch > conv, "architecture should decay slower than conversation");
    }
}
