use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::server::AppState;

pub fn lme_recall(state: &Arc<AppState>, args: Value) -> Result<Value, LmeError> {
    let hash = args
        .get("hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LmeError::Validation("missing required field: hash".into()))?;

    let memory = state.storage.get_by_hash(hash)?;

    // Reheating: update last_access (FR-MEM-04)
    let now = chrono::Utc::now().timestamp();
    let _ = state.storage.update_last_access(hash, now);

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
