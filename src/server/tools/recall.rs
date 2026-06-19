use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::server::AppState;
use crate::storage::models::MemoryUnit;

pub fn lme_recall(state: &Arc<AppState>, args: Value) -> Result<Value, LmeError> {
    let hash = args
        .get("hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LmeError::Validation("missing required field: hash".into()))?;

    let memory = match state.storage.get_by_hash(hash) {
        Ok(mem) => mem,
        Err(LmeError::NotFound(_)) => {
            // Fallback: hash may be corrupted in transit. Try prefix match within project.
            let project = args.get("project").and_then(|v| v.as_str());
            fallback_recall(state, hash, project)?
        }
        Err(e) => return Err(e),
    };

    // Reheating: update last_access (FR-MEM-04)
    let now = chrono::Utc::now().timestamp();
    let _ = state.storage.update_last_access(&memory.hash, now);

    let result = json!({
        "hash": memory.hash,
        "project": memory.project,
        "memory_type": memory.memory_type.as_str(),
        "essence": memory.essence,
        "summary": memory.summary,
        "facts": memory.facts,
        "source_ref": memory.source_ref,
        "sensitivity": memory.sensitivity.as_str(),
        "importance": memory.importance,
        "verified": memory.verified,
        "tags": memory.tags,
        "created_at": memory.created_at,
        "last_access": memory.last_access,
        "superseded_by": memory.superseded_by,
    });

    Ok(result)
}

/// Fallback: match by hash prefix within project.
/// When a hash gets corrupted in transit (e.g. 3-byte shift),
/// the uncorrupted prefix can still identify the correct memory.
fn fallback_recall(
    state: &Arc<AppState>,
    hash: &str,
    project: Option<&str>,
) -> Result<MemoryUnit, LmeError> {
    let proj = project.ok_or_else(|| {
        LmeError::NotFound(format!(
            "memory not found: {hash}. Tip: pass 'project' for fallback prefix lookup.",
        ))
    })?;

    let memories = state.storage.list_by_project(proj)?;

    // Find memory with longest common hash prefix
    let best = memories
        .into_iter()
        .filter_map(|m| {
            let prefix_len = m.hash
                .chars()
                .zip(hash.chars())
                .take_while(|(a, b)| a == b)
                .count();
            if prefix_len >= 16 {
                Some((prefix_len, m))
            } else {
                None
            }
        })
        .max_by_key(|(len, _)| *len);

    match best {
        Some((len, m)) => {
            tracing::debug!("recall: hash fallback matched via {}-char prefix", len);
            Ok(m)
        }
        None => Err(LmeError::NotFound(format!(
            "memory not found: {hash}. No prefix match (≥16 chars) in project '{proj}'.",
        ))),
    }
}
