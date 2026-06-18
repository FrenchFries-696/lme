//! Pruning logic (FR-DCY-04).
//! Memories below prune_threshold for >30 days are moved to archive table.
//! Currently manual-trigger only (deferred automatic timer to Phase 2).

use crate::decay::DecayEngine;
use crate::error::LmeError;
use crate::storage::Storage;

/// Move decayed memories to archive table.
/// Returns count of pruned memories.
pub fn prune_below_threshold(
    storage: &Storage,
    engine: &DecayEngine,
    project: &str,
    threshold: f64,
) -> Result<usize, LmeError> {
    let memories = storage.list_by_project(project)?;
    let now = chrono::Utc::now().timestamp();
    let mut pruned = 0usize;

    // Create archive table if not exists
    storage.conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memories_archive (
            hash TEXT PRIMARY KEY,
            project TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            memory_type TEXT NOT NULL,
            essence TEXT NOT NULL,
            summary TEXT,
            facts TEXT NOT NULL DEFAULT '[]',
            source_ref TEXT NOT NULL,
            sensitivity TEXT NOT NULL DEFAULT 'internal',
            importance INTEGER NOT NULL DEFAULT 3,
            verified INTEGER NOT NULL DEFAULT 0,
            embedding BLOB,
            embedding_model TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            created_at INTEGER NOT NULL,
            last_access INTEGER NOT NULL,
            superseded_by TEXT,
            archived_at INTEGER NOT NULL,
            vault_path TEXT,
            vault_hash TEXT
        )",
    ).map_err(|e| LmeError::Database(e))?;

    for mem in &memories {
        let score = engine.score(mem, now);
        if score < threshold {
            let days_since = ((now - mem.last_access) as f64) / 86_400.0;
            if days_since > 30.0 {
                // Move to archive
                let facts_json = serde_json::to_string(&mem.facts).unwrap_or_default();
                let tags_json = serde_json::to_string(&mem.tags).unwrap_or_default();
                let embedding_blob: Option<Vec<u8>> = mem.embedding.as_ref().map(|v| {
                    v.iter().flat_map(|f| f.to_le_bytes()).collect()
                });

                storage.conn.execute(
                    "INSERT OR IGNORE INTO memories_archive
                     (hash, project, owner_id, memory_type, essence, summary,
                      facts, source_ref, sensitivity, importance, verified,
                      embedding, embedding_model, tags, created_at, last_access,
                      superseded_by, archived_at, vault_path, vault_hash)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
                    rusqlite::params![
                        mem.hash, mem.project, mem.owner_id,
                        mem.memory_type.as_str(), mem.essence, mem.summary,
                        facts_json, mem.source_ref, mem.sensitivity.as_str(),
                        mem.importance, mem.verified as i32,
                        embedding_blob, mem.embedding_model, tags_json,
                        mem.created_at, mem.last_access, mem.superseded_by,
                        now, None::<String>, None::<String>,
                    ],
                ).map_err(|e| LmeError::Database(e))?;

                // Delete from main table
                storage.conn.execute(
                    "DELETE FROM memories WHERE hash = ?1",
                    rusqlite::params![mem.hash],
                ).map_err(|e| LmeError::Database(e))?;

                pruned += 1;
            }
        }
    }

    if pruned > 0 {
        tracing::info!("pruned {} memories from project '{}'", pruned, project);
    }

    Ok(pruned)
}
