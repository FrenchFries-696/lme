use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::server::AppState;
use crate::storage::models::Sensitivity;

pub fn lme_search(state: &Arc<AppState>, args: Value) -> Result<Value, LmeError> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LmeError::Validation("missing required field: query".into()))?;

    let project = args.get("project").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let results = state.storage.search_fts(query, project, limit)?;

    // Filter out secret memories
    let visible: Vec<_> = results
        .into_iter()
        .filter(|m| m.sensitivity != Sensitivity::Secret)
        .collect();

    // Rank by decay score
    let now = chrono::Utc::now().timestamp();
    let ranked = state.decay.rank(visible, now);

    // Build response with decay scores
    let items: Vec<Value> = ranked
        .into_iter()
        .map(|scored| {
            json!({
                "hash": scored.memory.hash,
                "essence": scored.memory.essence,
                "importance": scored.memory.importance,
                "memory_type": scored.memory.memory_type.as_str(),
                "score": scored.decay_score
            })
        })
        .collect();

    Ok(json!({ "results": items, "count": items.len() }))
}
