use crate::error::LmeError;
use crate::storage::Storage;

/// Detect conflicting memories: same project, overlapping facts, different values.
/// Returns hashes of memories that conflict with the new facts.
pub fn detect_conflicts(
    storage: &Storage,
    project: &str,
    new_hash: &str,
    new_facts: &[String],
) -> Result<Vec<String>, LmeError> {
    if new_facts.is_empty() {
        return Ok(vec![]);
    }

    let existing = storage.list_by_project(project)?;
    let mut conflicts = Vec::new();

    for mem in &existing {
        if mem.hash == new_hash {
            continue;
        }
        if mem.superseded_by.is_some() {
            continue; // Already superseded
        }

        // Check for overlapping fact subjects (conflict heuristic)
        let overlap = fact_overlap(new_facts, &mem.facts);
        if overlap {
            conflicts.push(mem.hash.clone());
        }
    }

    Ok(conflicts)
}

/// Simple overlap check: do any fact strings share the same subject key?
/// A "subject key" is the first word before any `:`, `=`, or `is` token.
fn fact_overlap(a: &[String], b: &[String]) -> bool {
    let subjects_a: Vec<String> = a.iter().map(|f| extract_subject(f)).collect();
    let subjects_b: Vec<String> = b.iter().map(|f| extract_subject(f)).collect();

    for sa in &subjects_a {
        if sa.is_empty() {
            continue;
        }
        for sb in &subjects_b {
            if sb.is_empty() {
                continue;
            }
            // Check if subjects share significant overlap
            if sa == sb
                || sa.contains(sb.as_str())
                || sb.contains(sa.as_str())
            {
                return true;
            }
        }
    }
    false
}

/// Extract the subject key from a fact string.
/// e.g. "user_limit=100" → "user_limit", "price is $5" → "price"
fn extract_subject(fact: &str) -> String {
    // Split on common separators
    for sep in &['=', ':', ' ', '\t'] {
        if let Some(pos) = fact.find(*sep) {
            let subject = fact[..pos].trim().to_lowercase();
            return subject.trim_matches(|c: char| c == '-' || c == '_').to_string();
        }
    }
    // No separator found — use first word as subject
    fact.split_whitespace()
        .next()
        .unwrap_or(fact)
        .trim()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_overlap_same_subject() {
        let a = vec!["user_limit=100".to_string()];
        let b = vec!["user_limit=200".to_string()];
        assert!(fact_overlap(&a, &b));
    }

    #[test]
    fn test_fact_overlap_different_subject() {
        let a = vec!["price=10".to_string()];
        let b = vec!["limit=50".to_string()];
        assert!(!fact_overlap(&a, &b));
    }

    #[test]
    fn test_extract_subject() {
        assert_eq!(extract_subject("user_limit=100"), "user_limit");
        assert_eq!(extract_subject("price is $5"), "price");
        assert_eq!(extract_subject("simple_fact"), "simple_fact");
    }
}
