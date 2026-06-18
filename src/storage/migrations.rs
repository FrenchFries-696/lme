use rusqlite::Connection;

use crate::error::LmeError;

const SCHEMA_V1: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;

CREATE TABLE IF NOT EXISTS memories (
    hash            TEXT PRIMARY KEY,
    project         TEXT NOT NULL,
    owner_id        TEXT NOT NULL,
    memory_type     TEXT NOT NULL CHECK(memory_type IN
                        ('conversation','knowledge','learning','decision','architecture')),
    essence         TEXT NOT NULL,
    summary         TEXT,
    facts           TEXT NOT NULL DEFAULT '[]',
    source_ref      TEXT NOT NULL,
    sensitivity     TEXT NOT NULL DEFAULT 'internal'
                        CHECK(sensitivity IN ('public','internal','secret')),
    importance      INTEGER NOT NULL DEFAULT 3 CHECK(importance BETWEEN 1 AND 5),
    verified        INTEGER NOT NULL DEFAULT 0,
    embedding       BLOB,
    embedding_model TEXT,
    tags            TEXT NOT NULL DEFAULT '[]',
    created_at      INTEGER NOT NULL,
    last_access     INTEGER NOT NULL,
    superseded_by   TEXT,
    vault_path      TEXT,
    vault_hash      TEXT
);

CREATE INDEX IF NOT EXISTS idx_mem_project  ON memories(project);
CREATE INDEX IF NOT EXISTS idx_mem_access   ON memories(last_access);
CREATE INDEX IF NOT EXISTS idx_mem_verified ON memories(verified);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    hash, essence, summary, facts, tags,
    content='memories',
    content_rowid='rowid',
    tokenize='unicode61'
);

-- Triggers to keep FTS5 index in sync
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, hash, essence, summary, facts, tags)
    VALUES (new.rowid, new.hash, new.essence, new.summary, new.facts, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, hash, essence, summary, facts, tags)
    VALUES ('delete', old.rowid, old.hash, old.essence, old.summary, old.facts, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, hash, essence, summary, facts, tags)
    VALUES ('delete', old.rowid, old.hash, old.essence, old.summary, old.facts, old.tags);
    INSERT INTO memories_fts(rowid, hash, essence, summary, facts, tags)
    VALUES (new.rowid, new.hash, new.essence, new.summary, new.facts, new.tags);
END;
"#;

pub fn run_migrations(conn: &Connection) -> Result<(), LmeError> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| {
        row.get(0)
    }).unwrap_or(0);

    if version < 1 {
        conn.execute_batch(SCHEMA_V1).map_err(|e| {
            LmeError::Database(e.into())
        })?;
        conn.pragma_update(None, "user_version", 1).map_err(|e| {
            LmeError::Database(e.into())
        })?;
        tracing::info!("migrations: applied v1");
    }

    Ok(())
}
