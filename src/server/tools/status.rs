use std::sync::Arc;
use serde_json::{json, Value};

use crate::error::LmeError;
use crate::server::AppState;

pub fn lme_status(state: &Arc<AppState>, args: Value) -> Result<Value, LmeError> {
    let project = args.get("project").and_then(|v| v.as_str());

    let total = if let Some(proj) = project {
        state.storage.count_by_project(proj)?
    } else {
        state.storage.count_all()?
    };

    let by_type = if let Some(proj) = project {
        state.storage.count_by_type(proj)?
    } else {
        Default::default()
    };

    let db_size = state.storage.db_size_bytes().unwrap_or(0);

    let by_type_json: Value = by_type
        .iter()
        .map(|(k, v)| (k.as_str(), json!(v)))
        .collect();

    #[cfg(feature = "embedding")]
    let embed_backend = {
        let e = state.embedder.lock().unwrap();
        if e.is_loaded() { e.info().name } else { "not-loaded".into() }
    };
    #[cfg(not(feature = "embedding"))]
    let embed_backend = "disabled";

    Ok(json!({
        "total": total,
        "by_type": by_type_json,
        "db_size_bytes": db_size,
        "db_health": "ok",
        "embed_backend": embed_backend
    }))
}
