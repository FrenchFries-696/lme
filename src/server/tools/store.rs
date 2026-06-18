use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::guardrails;
use crate::server::AppState;

pub fn lme_store(state: &Arc<AppState>, args: Value) -> Result<Value, LmeError> {
    let project = get_str(&args, "project")?;
    let memory_type_str = get_str(&args, "memory_type")?;
    let essence = get_str(&args, "essence")?;
    let source_ref = get_str(&args, "source_ref")?;

    // Validate memory_type
    if crate::storage::models::MemoryType::from_str(&memory_type_str).is_none() {
        return Err(LmeError::Validation(format!(
            "invalid memory_type: {}. Must be one of: conversation, knowledge, learning, decision, architecture",
            memory_type_str
        )));
    }

    let summary = args.get("summary").and_then(|v| v.as_str());
    let facts: Vec<String> = args
        .get("facts")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let importance = args
        .get("importance")
        .and_then(|v| v.as_u64())
        .map(|v| v as u8);

    let tags: Vec<String> = args
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let sensitivity = args
        .get("sensitivity")
        .and_then(|v| v.as_str())
        .and_then(|s| crate::storage::models::Sensitivity::from_str(s));

    // Use GuardedStore pipeline (FR-GRD-01 through FR-GRD-07, FR-CMP-04)
    let result = guardrails::GuardedStore::store(
        &state.storage,
        &essence,
        summary,
        &facts,
        &source_ref,
        &project,
        &memory_type_str,
        importance,
        &tags,
        sensitivity,
    )?;

    // Generate embedding if ONNX is available
    #[cfg(feature = "embedding")]
    if result.status == guardrails::StoreStatus::Stored
        || result.status == guardrails::StoreStatus::StoredAsInferred
    {
        let mut embedder = state.embedder.lock().unwrap();
        match embedder.embed(&essence) {
            Ok(vector) => {
                // Store embedding in DB
                let vec_ref: &[f32] = &vector;
                let blob: Vec<u8> = vec_ref.iter().flat_map(|f| f.to_le_bytes()).collect();
                state.storage.conn.execute(
                    "UPDATE memories SET embedding = ?1, embedding_model = ?2 WHERE hash = ?3",
                    rusqlite::params![blob, state.config.embedding.model, result.hash],
                ).ok();
                // Invalidate vector cache for this project
                embedder.invalidate_cache(&project);
                tracing::debug!("embedding generated for {}", result.hash);
            }
            Err(e) => {
                tracing::warn!("embedding failed: {} — continuing without vector", e);
            }
        }
    }

    let status_str = match result.status {
        guardrails::StoreStatus::Stored => "stored",
        guardrails::StoreStatus::Deduplicated => "deduplicated",
        guardrails::StoreStatus::Rejected => "rejected",
        guardrails::StoreStatus::StoredAsInferred => "stored-as-inferred",
    };

    Ok(json!({
        "hash": result.hash,
        "status": status_str,
        "verified": result.verified,
        "decay_score": result.decay_score,
        "conflicts": result.conflicts
    }))
}
fn get_str(args: &Value, key: &str) -> Result<String, LmeError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| LmeError::Validation(format!("missing required field: {}", key)))
}
