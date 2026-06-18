pub mod conflict;
pub mod dedup;
pub mod sensitivity;
pub mod source_ref;
pub mod verify;

use crate::error::LmeError;
use crate::storage::models::{MemoryType, Sensitivity, StoreInput};
use crate::storage::Storage;

/// Result of a guarded store operation.
#[derive(Debug, PartialEq)]
pub enum StoreStatus {
    Stored,
    Deduplicated,
    Rejected,
    StoredAsInferred,
}

pub struct StoreResult {
    pub hash: String,
    pub status: StoreStatus,
    pub verified: bool,
    pub decay_score: f64,
    pub conflicts: Vec<String>,
}

/// Stateless guardrail pipeline that wraps Store operations.
/// All methods take `&Storage` as first parameter.
pub struct GuardedStore;

impl GuardedStore {
    /// Full store pipeline:
    /// 1. Validate source_ref (FR-GRD-03)
    /// 2. Check anti-re-compression (FR-CMP-04)
    /// 3. Verify facts (FR-GRD-01/02)
    /// 4. Content hash + dedup (FR-GRD-04)
    /// 5. Conflict detection (FR-GRD-05)
    /// 6. Sensitivity default (FR-GRD-06)
    /// 7. Insert into storage
    pub fn store(
        storage: &Storage,
        essence: &str,
        summary: Option<&str>,
        facts: &[String],
        source_ref: &str,
        project: &str,
        memory_type: &str,
        importance: Option<u8>,
        tags: &[String],
    ) -> Result<StoreResult, LmeError> {
        // --- 1. Validate source_ref (FR-GRD-03) ---
        source_ref::validate_source_ref(source_ref)?;

        // --- 2. Anti-re-compression gate (FR-CMP-04) ---
        if source_ref::is_memory_hash(source_ref) {
            match storage.get_by_hash(source_ref.trim()) {
                Ok(_) => {
                    return Ok(StoreResult {
                        hash: String::new(),
                        status: StoreStatus::Rejected,
                        verified: false,
                        decay_score: 0.0,
                        conflicts: vec![],
                    });
                }
                Err(LmeError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }

        // --- 3. Verify facts (FR-GRD-01, FR-GRD-02) ---
        let (verified, _confidence) = verify::verify_facts(facts, source_ref);
        let importance_val = importance.unwrap_or(3);
        let decay_score = if verified {
            importance_val as f64
        } else {
            importance_val as f64 * 0.6
        };

        // --- 4. Content hash + dedup (FR-GRD-04) ---
        let mt = MemoryType::from_str(memory_type).unwrap_or(MemoryType::Knowledge);
        let sens = Sensitivity::from_str("internal").unwrap_or(Sensitivity::Secret);

        let input = StoreInput {
            project: project.to_string(),
            memory_type: mt,
            essence: essence.to_string(),
            summary: summary.map(String::from),
            facts: facts.to_vec(),
            source_ref: source_ref.to_string(),
            sensitivity: sens,
            importance: Some(importance_val),
            tags: tags.to_vec(),
        };

        let hash = Storage::content_hash(&input);

        match dedup::check_dedup(storage, &hash) {
            Ok(Some(existing)) => {
                let now = chrono::Utc::now().timestamp();
                storage.update_last_access(&existing, now)?;
                return Ok(StoreResult {
                    hash: existing,
                    status: StoreStatus::Deduplicated,
                    verified: true,
                    decay_score: importance_val as f64,
                    conflicts: vec![],
                });
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }

        // --- 5. Conflict detection (FR-GRD-05) ---
        let conflicts = conflict::detect_conflicts(storage, project, &hash, facts)?;

        // --- 6. Sensitivity default (FR-GRD-06) — in StoreInput default ---

        // --- 7. Insert ---
        let now = chrono::Utc::now().timestamp();
        storage.insert_memory(&input, None, now)?;

        for conflict_hash in &conflicts {
            let _ = storage.mark_superseded(conflict_hash, &hash);
        }

        let status = if verified {
            StoreStatus::Stored
        } else {
            StoreStatus::StoredAsInferred
        };

        Ok(StoreResult {
            hash,
            status,
            verified,
            decay_score,
            conflicts,
        })
    }
}
