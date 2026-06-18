use rusqlite::params;
use std::collections::HashMap;

use crate::error::LmeError;
use crate::storage::models::{MemoryType, MemoryUnit};

use super::Storage;

/// SQL columns for memory row selection from single-table queries.
const MEMORY_COLS: &str = "hash, project, owner_id, memory_type, essence, summary, \
    facts, source_ref, sensitivity, importance, verified, \
    embedding, embedding_model, tags, created_at, last_access, superseded_by";

/// Same columns with `m.` prefix for JOIN queries where `memories m` is aliased.
const MEMORY_COLS_M: &str = "m.hash, m.project, m.owner_id, m.memory_type, m.essence, \
    m.summary, m.facts, m.source_ref, m.sensitivity, m.importance, m.verified, \
    m.embedding, m.embedding_model, m.tags, m.created_at, m.last_access, m.superseded_by";

/// Map a rusqlite Row to a MemoryUnit (used as fn pointer for query_map).
fn map_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryUnit> {
    Storage::row_to_memory(row)
}

impl Storage {
    pub fn search_fts(
        &self,
        query: &str,
        project: Option<&str>,
        limit: usize,
    ) -> Result<Vec<MemoryUnit>, LmeError> {
        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(proj) = project {
            (
                format!(
                    "SELECT {} FROM memories_fts f \
                     JOIN memories m ON f.hash = m.hash \
                     WHERE memories_fts MATCH ?1 AND m.project = ?2 \
                     ORDER BY rank LIMIT {}",
                    MEMORY_COLS_M, limit
                ),
                vec![Box::new(query.to_string()), Box::new(proj.to_string())],
            )
        } else {
            (
                format!(
                    "SELECT {} FROM memories_fts f \
                     JOIN memories m ON f.hash = m.hash \
                     WHERE memories_fts MATCH ?1 \
                     ORDER BY rank LIMIT {}",
                    MEMORY_COLS_M, limit
                ),
                vec![Box::new(query.to_string())],
            )
        };

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self
            .conn
            .prepare(&sql)
            .map_err(|e| LmeError::Database(e))?;

        let rows: Vec<MemoryUnit> = stmt
            .query_map(params_ref.as_slice(), map_row)
            .map_err(|e| LmeError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    pub fn list_by_project(
        &self,
        project: &str,
    ) -> Result<Vec<MemoryUnit>, LmeError> {
        let mut stmt = self
            .conn
            .prepare(&format!(
                "SELECT {} FROM memories WHERE project = ?1 ORDER BY last_access DESC",
                MEMORY_COLS
            ))
            .map_err(|e| LmeError::Database(e))?;

        let rows: Vec<MemoryUnit> = stmt
            .query_map(params![project], map_row)
            .map_err(|e| LmeError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    pub fn count_by_project(&self, project: &str) -> Result<i64, LmeError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM memories WHERE project = ?1",
                params![project],
                |row| row.get(0),
            )
            .map_err(|e| LmeError::Database(e))
    }

    pub fn count_all(&self) -> Result<i64, LmeError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .map_err(|e| LmeError::Database(e))
    }

    pub fn count_by_type(
        &self,
        project: &str,
    ) -> Result<HashMap<MemoryType, i64>, LmeError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT memory_type, COUNT(*) FROM memories \
                 WHERE project = ?1 GROUP BY memory_type",
            )
            .map_err(|e| LmeError::Database(e))?;

        let rows = stmt
            .query_map(params![project], |row| {
                let t: String = row.get(0)?;
                let c: i64 = row.get(1)?;
                Ok((t, c))
            })
            .map_err(|e| LmeError::Database(e))?;

        let mut map = HashMap::new();
        for row in rows {
            if let Ok((type_str, count)) = row {
                if let Some(mt) = MemoryType::from_str(&type_str) {
                    map.insert(mt, count);
                }
            }
        }

        // Ensure all types appear even if count is 0
        for mt in MemoryType::all() {
            map.entry(mt).or_insert(0);
        }

        Ok(map)
    }

    pub fn get_all_embeddings(
        &self,
        project: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, LmeError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT hash, embedding FROM memories \
                 WHERE project = ?1 AND embedding IS NOT NULL",
            )
            .map_err(|e| LmeError::Database(e))?;

        let rows: Vec<(String, Vec<f32>)> = stmt
            .query_map(params![project], |row| {
                let hash: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                let vec: Vec<f32> = blob
                    .chunks(4)
                    .map(|chunk| {
                        let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                        f32::from_le_bytes(arr)
                    })
                    .collect();
                Ok((hash, vec))
            })
            .map_err(|e| LmeError::Database(e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    pub fn db_size_bytes(&self) -> Result<u64, LmeError> {
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .map_err(|e| LmeError::Database(e))?;
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .map_err(|e| LmeError::Database(e))?;

        Ok((page_count * page_size) as u64)
    }
}
