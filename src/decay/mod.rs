use crate::config::DecayConfig;
use crate::storage::models::{MemoryType, MemoryUnit};

pub mod formula;

pub struct DecayEngine {
    config: DecayConfig,
}

pub struct ScoredMemory {
    pub memory: MemoryUnit,
    pub decay_score: f64,
}

impl DecayEngine {
    pub fn new(config: DecayConfig) -> Self {
        Self { config }
    }

    /// Calculate decay score for a single memory at query time.
    pub fn score(&self, memory: &MemoryUnit, now: i64) -> f64 {
        let days_since_access = ((now - memory.last_access) as f64) / 86_400.0;
        let lambda = self.lambda_for(&memory.memory_type);

        formula::decay_score(
            memory.importance,
            lambda,
            days_since_access.max(0.0),
            memory.verified,
            self.config.conf_inferred,
        )
    }

    /// Sort and score all memories for a project.
    /// Returns scored memories sorted by decay_score descending,
    /// ties broken by newer last_access.
    pub fn rank(&self, memories: Vec<MemoryUnit>, now: i64) -> Vec<ScoredMemory> {
        let mut scored: Vec<ScoredMemory> = memories
            .into_iter()
            .map(|m| {
                let score = self.score(&m, now);
                ScoredMemory {
                    memory: m,
                    decay_score: score,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.decay_score
                .partial_cmp(&a.decay_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.memory.last_access.cmp(&a.memory.last_access))
        });

        // All-decayed fallback: if all scores < 0.01, sort by raw importance
        let all_near_zero = scored.iter().all(|s| s.decay_score < 0.01);
        if all_near_zero && !scored.is_empty() {
            scored.sort_by(|a, b| {
                b.memory.importance
                    .cmp(&a.memory.importance)
                    .then_with(|| b.memory.last_access.cmp(&a.memory.last_access))
            });
            // Update scores to reflect fallback ranking
            for (i, s) in scored.iter_mut().enumerate() {
                // Assign synthetic scores decreasing from 5.0
                s.decay_score = 5.0 - (i as f64 * 0.1).min(4.9);
            }
        }

        scored
    }

    pub fn lambda_for(&self, memory_type: &MemoryType) -> f64 {
        match memory_type {
            MemoryType::Conversation => self.config.lambda_conversation,
            MemoryType::Knowledge => self.config.lambda_knowledge,
            MemoryType::Learning => self.config.lambda_learning,
            MemoryType::Decision => self.config.lambda_decision,
            MemoryType::Architecture => self.config.lambda_architecture,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::{MemoryType, MemoryUnit};

    fn make_memory(importance: u8, verified: bool, days_ago: i64) -> MemoryUnit {
        let now = chrono::Utc::now().timestamp();
        MemoryUnit {
            hash: format!("test-{}", importance),
            project: "test".into(),
            owner_id: "test".into(),
            memory_type: MemoryType::Knowledge,
            essence: "test".into(),
            summary: None,
            facts: vec![],
            source_ref: "test".into(),
            sensitivity: crate::storage::models::Sensitivity::Internal,
            importance,
            verified,
            embedding: None,
            embedding_model: None,
            tags: vec![],
            created_at: now - days_ago * 86_400,
            last_access: now - days_ago * 86_400,
            superseded_by: None,
        }
    }

    #[test]
    fn test_rank_new_ranks_higher() {
        let engine = DecayEngine::new(DecayConfig::default());
        let now = chrono::Utc::now().timestamp();
        let new = make_memory(3, true, 0);
        let old = make_memory(3, true, 100);

        let ranked = engine.rank(vec![old, new], now);
        assert_eq!(ranked[0].memory.last_access, now); // new should be first
    }

    #[test]
    fn test_all_decayed_fallback() {
        let engine = DecayEngine::new(DecayConfig::default());
        let now = chrono::Utc::now().timestamp();
        let low = make_memory(2, false, 100_000);
        let high = make_memory(5, false, 100_000);

        let ranked = engine.rank(vec![low, high], now);
        // Both are near-zero, should fall back to importance
        assert_eq!(ranked[0].memory.importance, 5);
    }
}
