# CLAUDE.md

Project: lme — Local Memory Engine for AI Agents.

## Build

```bash
cargo build --release --features embedding   # full build with ONNX
cargo build --release                        # lightweight, FTS5-only
cargo test                                   # run tests
```

## Architecture

```
src/
├── main.rs           Entrypoint, CLI subcommands, tracing init, server run
├── lib.rs            Module declarations
├── config.rs         lme.toml parsing (Config, DecayConfig, EmbeddingConfig, etc.)
├── error.rs          LmeError enum (Config, Database, Embedding, Validation, NotFound, Internal)
├── server/
│   ├── mod.rs        MCP stdio JSON-RPC loop, AppState, handle_message dispatch
│   ├── protocol.rs   JsonRpcRequest/JsonRpcResponse types
│   ├── transport.rs  Line-based stdio framing
│   └── tools/        5 tool handlers (store, context, search, recall, status)
├── storage/
│   ├── mod.rs        Storage struct, open(), open_in_memory(), WAL + FK pragmas
│   ├── models.rs     MemoryUnit, StoreInput, MemoryType enum, Sensitivity enum
│   ├── migrations.rs Schema v1: memories table + FTS5 virtual table + triggers
│   ├── crud.rs       Insert, get_by_hash, update_last_access, mark_superseded, content_hash
│   └── search.rs     FTS5 search, list_by_project, count, get_all_embeddings
├── guardrails/
│   ├── mod.rs        GuardedStore pipeline orchestration (7-step store pipeline)
│   ├── verify.rs     Rule-based fact verification
│   ├── dedup.rs      SHA-256 content hash dedup
│   ├── conflict.rs   Conflicting memory detection
│   ├── source_ref.rs Provenance validation + anti-re-compression gate
│   └── sensitivity.rs Default sensitivity classification
├── decay/
│   ├── mod.rs        DecayEngine, ScoredMemory, rank(), score(), all-decayed fallback
│   ├── formula.rs    decay_score = importance * exp(-lambda * days) * conf
│   └── prune.rs      Low-score archive pruning
├── context/
│   └── packer.rs     Fidelity degradation: facts → summary → essence within char budget
├── embedder/         (feature-gated: "embedding")
│   ├── mod.rs        Embedder struct, lazy load, vector cache, invalidate_cache
│   ├── onnx.rs       ONNX Runtime session + inference
│   ├── tokenize.rs   HuggingFace tokenizer wrapper
│   └── cosine.rs     Brute-force cosine similarity search
└── download.rs       HuggingFace model downloader (feature-gated)
```

## Key Design Decisions

- **SQLite + FTS5**: single-file DB, no external deps, WAL mode for concurrent reads
- **Decay formula**: exponential decay per memory type, different lambdas (arch=0.001 slowest, conv=0.01 fastest)
- **Fidelity degradation**: context packer degrades from full facts → summary → essence when budget tight
- **Guardrail pipeline**: source_ref validation → anti-re-compression → fact verify → dedup → conflict detect → sensitivity default → insert
- **Content hash**: SHA-256 of essence.lowercase() + sorted facts + source_ref + project
- **Verified flag**: rule-based fact verification checks dates/numbers/names against source_ref
- **Secret isolation**: sensitivity=secret memories excluded from context/search results
- **All-decayed fallback**: when all scores < 0.01, rank by raw importance instead

## Code Standards

- Rust 2021 edition, thiserror for error types, serde for serialization
- rusqlite with bundled SQLite, rusqlite::params! for parameterized queries
- Feature gate: `#[cfg(feature = "embedding")]` for ONNX-dependent code
- Tests use in-memory SQLite (`Storage::open_in_memory()`)
- Logging via tracing to stderr (stdout reserved for MCP protocol)
- Tool handlers return `Result<Value, LmeError>`, dispatched by name in handle_call_tool

## Config

- Config loaded from `lme.toml` in CWD, or `LME_CONFIG` env var
- All config sections have Default impls with sensible values
- Store/context triggers configurable in `[store]` section
