/// Rule-based fact verification (FR-GRD-01, FR-GRD-02).
///
/// Checks if extractive facts (numbers, dates, proper nouns, IDs)
/// appear verbatim in the source_ref text.
///
/// Returns (verified: bool, confidence: f64) where:
/// - `verified=true` if >= 50% of checked facts match
/// - `confidence`: fraction of matched facts (0.0 to 1.0)
pub fn verify_facts(facts: &[String], source_ref: &str) -> (bool, f64) {
    if facts.is_empty() {
        return (true, 1.0); // No facts to verify = trivially verified
    }

    let source_lower = source_ref.to_lowercase();
    let mut matched = 0u32;
    let mut checked = 0u32;

    for fact in facts {
        let fact_trimmed = fact.trim();
        if fact_trimmed.is_empty() {
            continue;
        }
        checked += 1;

        // Extract checkable parts from the fact
        let checkable = extract_checkable_items(fact_trimmed);

        if checkable.is_empty() {
            // Fact contains no checkable items — count as matched
            // (narrative facts like "system should be fast" can't be verified)
            matched += 1;
            continue;
        }

        // Check each item against source_ref
        let fact_matched = checkable
            .iter()
            .any(|item| source_lower.contains(&item.to_lowercase()));

        if fact_matched {
            matched += 1;
        }
    }

    if checked == 0 {
        return (true, 1.0);
    }

    let confidence = matched as f64 / checked as f64;
    let verified = confidence >= 0.5; // 50% threshold

    (verified, confidence)
}

/// Extract checkable items from a fact string:
/// numbers, dates, proper nouns, identifiers.
fn extract_checkable_items(fact: &str) -> Vec<String> {
    let mut items = Vec::new();

    // Extract numbers (integers, decimals, percentages)
    for word in fact.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':');
        if cleaned.chars().any(|c| c.is_numeric())
            && cleaned.chars().filter(|c| c.is_numeric()).count() >= 2
        {
            items.push(cleaned.to_string());
        }
    }

    // Extract potential dates (YYYY-MM-DD, DD/MM/YYYY)
    for word in fact.split_whitespace() {
        if word.chars().filter(|c| *c == '-' || *c == '/').count() >= 2
            && word.chars().any(|c| c.is_numeric())
        {
            items.push(word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '/').to_string());
        }
    }

    // Extract capitalized proper nouns (names, IDs)
    for word in fact.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
        if cleaned.len() >= 3
            && cleaned.chars().next().map_or(false, |c| c.is_uppercase())
            && cleaned.chars().all(|c| c.is_alphanumeric())
        {
            items.push(cleaned.to_string());
        }
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_facts_with_number_in_source() {
        let facts = vec!["user limit is 100".to_string()];
        let source = "The system has a user limit of 100 accounts";
        let (verified, conf) = verify_facts(&facts, source);
        assert!(verified);
        assert!(conf > 0.0);
    }

    #[test]
    fn test_verify_facts_missing_from_source() {
        let facts = vec!["price is $999".to_string()];
        let source = "We discussed the new feature requirements";
        let (verified, conf) = verify_facts(&facts, source);
        assert!(!verified);
    }

    #[test]
    fn test_verify_empty_facts() {
        let (verified, conf) = verify_facts(&[], "any source");
        assert!(verified);
        assert_eq!(conf, 1.0);
    }

    #[test]
    fn test_extract_numbers() {
        let items = extract_checkable_items("limit is 500 users");
        assert!(items.contains(&"500".to_string()));
    }
}
