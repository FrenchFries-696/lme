pub mod crud;
pub mod migrations;
pub mod models;
pub mod search;

use std::path::Path;

use rusqlite::Connection;

use crate::config::DatabaseConfig;
use crate::error::LmeError;

pub struct Storage {
    pub conn: Connection,
}

impl Storage {
    pub fn open(path: &Path, config: &DatabaseConfig) -> Result<Self, LmeError> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|_e| {
                    LmeError::Database(rusqlite::Error::InvalidPath(
                        parent.to_path_buf(),
                    ))
                })?;
            }
        }

        let conn = Connection::open(path).map_err(|e| LmeError::Database(e))?;

        // Enable WAL mode
        if config.wal_mode {
            conn.pragma_update(None, "journal_mode", "WAL")
                .map_err(|e| LmeError::Database(e))?;
        }

        // Enable foreign keys
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| LmeError::Database(e))?;

        // Run integrity check before migrations
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|e| LmeError::Database(e))?;

        if integrity != "ok" {
            return Err(LmeError::Database(rusqlite::Error::InvalidQuery));
        }

        let storage = Storage { conn };

        // Run migrations
        migrations::run_migrations(&storage.conn)?;

        Ok(storage)
    }

    /// Open an in-memory database for testing.
    pub fn open_in_memory() -> Result<Self, LmeError> {
        let conn =
            Connection::open_in_memory().map_err(|e| LmeError::Database(e))?;
        let storage = Storage { conn };
        migrations::run_migrations(&storage.conn)?;
        Ok(storage)
    }
}
