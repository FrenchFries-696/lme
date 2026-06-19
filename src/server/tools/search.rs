use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::server::AppState;
use crate::storage::models::{MemoryUnit, Sensitivity};

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
    let mode = args
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");

    let results = match mode {
        "semantic" => semantic_search(state, query, project, limit)?,
        "keyword" => state.storage.search_fts(query, project, limit)?,
        _ => {
            // auto: try semantic first, fallback to FTS5
            match semantic_search(state, query, project, limit) {
                Ok(results) if !results.is_empty() => results,
                Ok(_) => {
                    tracing::debug!("semantic: 0 results, falling back to FTS5");
                    state.storage.search_fts(query, project, limit)?
                }
                Err(e) => {
                    tracing::debug!("semantic: {} — falling back to FTS5", e);
                    state.storage.search_fts(query, project, limit)?
                }
            }
        }
    };

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
                "summary": scored.memory.summary,
                "facts": scored.memory.facts,
                "importance": scored.memory.importance,
                "memory_type": scored.memory.memory_type.as_str(),
                "score": scored.decay_score
            })
        })
        .collect();

    Ok(json!({ "results": items, "count": items.len() }))
}

/// Semantic search via ONNX embedding cosine similarity.
/// Feature-gated: when embedding is disabled, returns an error for explicit "semantic" mode,
/// and auto mode falls back to FTS5 via the caller.
#[allow(unused_variables)]
fn semantic_search(
    state: &Arc<AppState>,
    query: &str,
    project: Option<&str>,
    limit: usize,
) -> Result<Vec<MemoryUnit>, LmeError> {
    #[cfg(not(feature = "embedding"))]
    {
        // Called from "semantic" mode explicitly
        return Err(LmeError::Validation(
            "semantic mode requires --features embedding. Build with embedding enabled.".into(),
        ));
    }

    #[cfg(feature = "embedding")]
    {
        let proj = project.ok_or_else(|| {
            LmeError::Validation("semantic search requires a project filter".into())
        })?;

        let mut embedder = state.embedder.lock().unwrap();

        // Embed the query
        let query_vec = embedder.embed(query)?;

        // Try cosine search against cached vectors
        let matches = embedder.cosine_search(&query_vec, proj, limit);

        // If cache miss, load vectors from DB and retry
        let matches = if matches.is_empty() {
            let vectors = state.storage.get_all_embeddings(proj)?;
            if vectors.is_empty() {
                return Ok(Vec::new());
            }
            embedder.cache_vectors(proj, vectors);
            embedder.cosine_search(&query_vec, proj, limit)
        } else {
            matches
        };

        // Load full MemoryUnits by hash
        let memories: Vec<MemoryUnit> = matches
            .into_iter()
            .filter_map(|(hash, _)| state.storage.get_by_hash(&hash).ok())
            .collect();

        Ok(memories)
    }
}
