//! Cosine similarity brute-force search.
//! Input vectors assumed L2-normalized (dot product = cosine similarity).

/// Brute-force top-K cosine search over candidates.
/// Returns (hash, score) sorted by score descending.
pub fn cosine_search(
    query: &[f32],
    candidates: &[(String, Vec<f32>)],
    top_k: usize,
) -> Vec<(String, f32)> {
    let mut scored: Vec<(String, f32)> = candidates
        .iter()
        .map(|(hash, vec)| {
            let score = dot_product(query, vec);
            (hash.clone(), score)
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.truncate(top_k);
    scored
}

/// Dot product of two equal-length float slices.
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_search() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            ("a".into(), vec![1.0, 0.0, 0.0]), // perfect match
            ("b".into(), vec![0.0, 1.0, 0.0]), // orthogonal
            ("c".into(), vec![0.7, 0.7, 0.0]), // partial
        ];
        let results = cosine_search(&query, &candidates, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "a");
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }
}
