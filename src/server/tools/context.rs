use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::server::AppState;
use crate::storage::models::Sensitivity;

pub fn lme_context(state: &Arc<AppState>, args: Value) -> Result<Value, LmeError> {
    let project = args
        .get("project")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LmeError::Validation("missing required field: project".into()))?;

    let char_budget = args
        .get("char_budget")
        .and_then(|v| v.as_u64())
        .unwrap_or(state.config.limits.max_context_chars as u64) as usize;

    let memories = state.storage.list_by_project(project)?;

    // Filter out secret memories
    let visible: Vec<_> = memories
        .into_iter()
        .filter(|m| m.sensitivity != Sensitivity::Secret)
        .collect();

    // Rank by decay score (Phase 6)
    let now = chrono::Utc::now().timestamp();
    let ranked = state.decay.rank(visible, now);

    // Build context with fidelity degradation
    let (context, used, n_units) = crate::context::packer::build_context(&ranked, char_budget);

    // Reheat: update last_access for all included memories
    for scored in ranked.iter().take(n_units) {
        let _ = state.storage.update_last_access(&scored.memory.hash, now);
    }

    Ok(json!({
        "context": context,
        "used_chars": used,
        "n_units": n_units
    }))
}
