use rusqlite::params;
use sha2::{Digest, Sha256};

use crate::error::LmeError;
use crate::storage::models::{MemoryType, MemoryUnit, Sensitivity, StoreInput};

use super::Storage;

impl Storage {
    /// Compute content hash for dedup (FR-GRD-04).
    /// Normalizes: essence.trim().to_lowercase() + sorted facts + source_ref + project
    pub fn content_hash(input: &StoreInput) -> String {
        let mut hasher = Sha256::new();
        hasher.update(input.essence.trim().to_lowercase().as_bytes());
        let mut sorted_facts = input.facts.clone();
        sorted_facts.sort();
        for f in &sorted_facts {
            hasher.update(f.as_bytes());
            hasher.update(b"|");
        }
        hasher.update(input.source_ref.trim().as_bytes());
        hasher.update(input.project.trim().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn insert_memory(
        &self,
        input: &StoreInput,
        embedding: Option<&[f32]>,
        now: i64,
    ) -> Result<String, LmeError> {
        let hash = Self::content_hash(input);

        // Dedup check (FR-GRD-04)
        let exists: bool = self.conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM memories WHERE hash = ?1",
                params![hash],
                |row| row.get(0),
            )
            .map_err(|e| LmeError::Database(e))?;

        if exists {
            // Update last_access only (reheating on duplicate store)
            self.conn
                .execute(
                    "UPDATE memories SET last_access = ?1 WHERE hash = ?2",
                    params![now, hash],
                )
                .map_err(|e| LmeError::Database(e))?;
            return Ok(hash);
        }

        let facts_json = serde_json::to_string(&input.facts)
            .map_err(|e| LmeError::Internal(format!("facts serialization: {}", e)))?;
        let tags_json = serde_json::to_string(&input.tags)
            .map_err(|e| LmeError::Internal(format!("tags serialization: {}", e)))?;
        let importance = input.importance.unwrap_or(3);

        let embedding_blob: Option<Vec<u8>> = embedding.map(|v| {
            v.iter().flat_map(|f| f.to_le_bytes()).collect()
        });

        self.conn
            .execute(
                r#"INSERT INTO memories (
                    hash, project, owner_id, memory_type, essence, summary,
                    facts, source_ref, sensitivity, importance, verified,
                    embedding, embedding_model, tags, created_at, last_access
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, NULL, ?12, ?13, ?14)"#,
                params![
                    hash,
                    input.project,
                    "config-user", // owner_id from config enforced at tool layer (Phase 3)
                    input.memory_type.as_str(),
                    input.essence,
                    input.summary,
                    facts_json,
                    input.source_ref,
                    input.sensitivity.as_str(),
                    importance,
                    embedding_blob,
                    tags_json,
                    now,
                    now,
                ],
            )
            .map_err(|e| LmeError::Database(e))?;

        Ok(hash)
    }

    pub fn get_by_hash(&self, hash: &str) -> Result<MemoryUnit, LmeError> {
        self.conn
            .query_row(
                "SELECT hash, project, owner_id, memory_type, essence, summary,
                        facts, source_ref, sensitivity, importance, verified,
                        embedding, embedding_model, tags, created_at,
                        last_access, superseded_by
                 FROM memories WHERE hash = ?1",
                params![hash],
                |row| Self::row_to_memory(row),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    LmeError::NotFound(format!("memory not found: {}", hash))
                }
                other => LmeError::Database(other),
            })
    }

    pub fn update_last_access(&self, hash: &str, timestamp: i64) -> Result<(), LmeError> {
        let rows = self
            .conn
            .execute(
                "UPDATE memories SET last_access = ?1 WHERE hash = ?2",
                params![timestamp, hash],
            )
            .map_err(|e| LmeError::Database(e))?;
        if rows == 0 {
            return Err(LmeError::NotFound(format!("memory not found: {}", hash)));
        }
        Ok(())
    }

    pub fn mark_superseded(
        &self,
        hash: &str,
        superseded_by: &str,
    ) -> Result<(), LmeError> {
        let rows = self
            .conn
            .execute(
                "UPDATE memories SET superseded_by = ?1 WHERE hash = ?2",
                params![superseded_by, hash],
            )
            .map_err(|e| LmeError::Database(e))?;
        if rows == 0 {
            return Err(LmeError::NotFound(format!(
                "memory not found: {}",
                hash
            )));
        }
        Ok(())
    }
}

impl Storage {
    pub(crate) fn row_to_memory(
        row: &rusqlite::Row,
    ) -> rusqlite::Result<MemoryUnit> {
        // Column order must match schema + SELECT queries:
        // 0:hash 1:project 2:owner_id 3:memory_type 4:essence 5:summary
        // 6:facts 7:source_ref 8:sensitivity 9:importance 10:verified
        // 11:embedding 12:embedding_model 13:tags 14:created_at
        // 15:last_access 16:superseded_by
        let facts_str: String = row.get(6)?;
        let facts: Vec<String> =
            serde_json::from_str(&facts_str).unwrap_or_default();

        let tags_str: String = row.get(13)?;
        let tags: Vec<String> =
            serde_json::from_str(&tags_str).unwrap_or_default();

        let sensitivity_str: String = row.get(8)?;
        let sensitivity = Sensitivity::from_str(&sensitivity_str)
            .unwrap_or(Sensitivity::Internal);

        let memory_type_str: String = row.get(3)?;
        let memory_type = MemoryType::from_str(&memory_type_str)
            .unwrap_or(MemoryType::Knowledge);

        let embedding_blob: Option<Vec<u8>> = row.get(11)?;
        let embedding: Option<Vec<f32>> = embedding_blob.map(|b| {
            b.chunks(4)
                .map(|chunk| {
                    let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                    f32::from_le_bytes(arr)
                })
                .collect()
        });

        Ok(MemoryUnit {
            hash: row.get(0)?,
            project: row.get(1)?,
            owner_id: row.get(2)?,
            memory_type,
            essence: row.get(4)?,
            summary: row.get(5)?,
            facts,
            source_ref: row.get(7)?,
            sensitivity,
            importance: row.get(9)?,
            verified: row.get::<_, i32>(10)? != 0,
            embedding,
            embedding_model: row.get(12)?,
            tags,
            created_at: row.get(14)?,
            last_access: row.get(15)?,
            superseded_by: row.get(16)?,
        })
    }
}
