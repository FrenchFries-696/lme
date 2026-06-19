# lme — Local Memory Engine

Long-term memory for AI agents. 100% local. No cloud.

Agent remembers projects, decisions, architecture across sessions. Works with Claude Code, Claude Desktop, any MCP client.

---

## How it works

```
┌──────────────┐     MCP stdio     ┌──────────────────────────────────┐
│  Claude / LLM │◄─────────────────►│  lme engine (your machine)       │
│  (cloud)      │                  │                                  │
│               │  store facts     │  ┌──────────┐   ┌─────────────┐  │
│  distills     │◄───────────────►│  │guardrails│──►│  embedder   │  │
│  info into    │                  │  │ verify   │   │  ONNX bge-m3│  │
│  structured   │  context/summary │  │ dedup    │   │  1024-dim   │  │
│  memories     │◄───────────────►│  └────┬─────┘   └──────┬──────┘  │
│               │                  │       │                │         │
│               │                  │  ┌────▼────────────────▼──────┐  │
│               │                  │  │       SQLite + FTS5       │  │
│               │                  │  │   memories | vectors |     │  │
│               │                  │  │   full-text index          │  │
│               │                  │  └────────────┬──────────────┘  │
│               │                  │               │                 │
│               │                  │  ┌────────────▼──────────────┐  │
│               │                  │  │     decay engine          │  │
│               │                  │  │  rank by time × importance│  │
│               │                  │  │  × verification confidence │  │
│               │                  │  └───────────────────────────┘  │
└──────────────┘                  └──────────────────────────────────┘
```

**Store path:** Agent distills conversation → guardrails validate provenance + verify facts → embed to 1024-dim vector → store in SQLite.

**Context path:** List all memories for project → rank by decay score → pack within char budget (full facts → summary → essence) → return to agent.

**Search path:** Query → hybrid search (cosine vector + FTS5 keyword) → re-rank by decay → filter secrets → return results.

### Decay formula

Each memory decays over time at different rates depending on type:

```
decay_score = importance × e^(-λ × days_since_access) × confidence

λ (decay rate):    architecture=0.001  (slowest, years)
                   decision=0.002
                   knowledge=0.005
                   learning=0.005
                   conversation=0.01  (fastest, months)

confidence:        1.0 for verified facts
                   0.6 for inferred/unverified
```

Recalling a memory resets its decay clock (reheating).

### Guardrails

Every `lme_store` runs through mandatory checks:

| Gate | What it does |
|------|--------------|
| Source validation | `source_ref` must be non-empty valid path/URL |
| Fact verification | Numbers, dates, names in `facts` checked against source |
| Dedup | SHA-256 content hash — duplicate updates timestamp only |
| Anti-re-compression | Cannot distill from a previously distilled memory |
| Conflict detection | Same subject, conflicting values → keeps both, marks `superseded_by` |
| Secret isolation | `sensitivity=secret` excluded from context/search results |

### Memory types

| Type | Use for | Decay speed |
|------|---------|-------------|
| `architecture` | System design, patterns, conventions | Slowest |
| `decision` | Why we chose X over Y, tradeoffs | Slow |
| `knowledge` | Domain facts, API behavior, docs | Medium |
| `learning` | Lessons learned, gotchas, tips | Medium |
| `conversation` | Meeting notes, chat summaries | Fastest |

---

## Install

### Prerequisites

- macOS (Apple Silicon). Linux/Intel works but not tested.
- Rust 1.85+ (`rustup update`)
- [ONNX Runtime](https://onnxruntime.ai) (`brew install onnxruntime`)

### Build

```bash
git clone https://github.com/FrenchFries-696/lme.git
cd lme

# Create your config from the example
cp lme.toml.example lme.toml

# Install ONNX Runtime (one time)
brew install onnxruntime

# Build with embeddings
cargo build --release --features embedding

# Download AI models (~560 MB, one time)
cargo run --release --features embedding -- download-models
```

Skip `--features embedding` for FTS5-only mode (no AI model needed).

### Register with Claude Code

```bash
claude mcp add -e LME_CONFIG=/absolute/path/to/lme/lme.toml lme -- /absolute/path/to/lme/target/release/lme
```

Example:

```bash
claude mcp add -e LME_CONFIG=$PWD/lme.toml lme -- $PWD/target/release/lme
```

Restart Claude Code. Done.

### Verify

Start Claude Code, ask: "use lme_status to check the memory engine".

Claude replies with engine stats: total memories, database size, model backend.

---

#
## Config

Copy `lme.toml.example` → `lme.toml` and edit to match your setup. Paths are relative to the project root — no changes needed if you run lme from this directory.

To use absolute paths (e.g. for `LME_CONFIG` env var pointing elsewhere):

```toml
[user]
owner_id = "your-name"

[database]
path = "./data/lme.db"
wal_mode = true

[embedding]
model = "bge-m3"
model_path = "./models/bge-m3-int8.onnx"
tokenizer_path = "./models/bge-m3-tokenizer.json"
lazy_load = true

[decay]
lambda_conversation = 0.01
lambda_architecture = 0.001
conf_inferred = 0.6
prune_threshold = 0.01

[limits]
max_context_chars = 50000
max_search_results = 20

[store]
store_triggers = ["remember", "save this", "store memory"]
context_triggers = ["recall context", "what do we know", "load context"]
```

---

## Tools

Claude calls these automatically. No manual invocation needed.

| Tool | What it does |
|------|--------------|
| `lme_store` | Save structured memory (essence, facts, source_ref) |
| `lme_context` | Load ranked project memory within char budget |
| `lme_search` | Hybrid search — cosine vector + FTS5 keyword |
| `lme_recall` | Pull up full memory by hash, resets decay clock |
| `lme_status` | Engine health: counts, DB size, model backend |

---

## Architecture

```
src/
├── main.rs                     Entrypoint, CLI (download-models, --version)
├── config.rs                   lme.toml parsing
├── error.rs                    Error types (thiserror)
├── server/                     MCP stdio JSON-RPC server
│   ├── mod.rs                  Main loop, AppState, message dispatch
│   ├── protocol.rs             JSON-RPC request/response types
│   ├── transport.rs            Line-based stdio framing
│   └── tools/                  5 tool handlers
│       ├── store.rs            Guarded store pipeline + embedding
│       ├── context.rs          Decay-ranked context builder
│       ├── search.rs           Hybrid search (FTS5 + vector)
│       ├── recall.rs           Memory recall with reheating
│       └── status.rs           Engine health + stats
├── storage/                    SQLite persistence
│   ├── mod.rs                  Storage struct, open, open_in_memory
│   ├── models.rs               MemoryUnit, StoreInput, enums
│   ├── migrations.rs           Schema v1, WAL, FTS5 triggers
│   ├── crud.rs                 Insert, get, update_last_access
│   └── search.rs               FTS5 search, list, count, embeddings
├── guardrails/                 Data integrity pipeline
│   ├── mod.rs                  GuardedStore orchestration
│   ├── verify.rs               Fact verification (rule-based)
│   ├── dedup.rs                Content-addressable hash check
│   ├── conflict.rs             Conflicting memory detection
│   ├── source_ref.rs           Provenance validation
│   └── sensitivity.rs          Secret classification + gates
├── decay/                      Time-based memory ranking
│   ├── mod.rs                  DecayEngine, ScoredMemory
│   ├── formula.rs              decay_score function
│   └── prune.rs                Low-score archive pruning
├── context/                    Context assembly
│   └── packer.rs               Fidelity degradation packing
├── embedder/                   ONNX model integration (feature-gated)
│   ├── mod.rs                  Embedder struct, lazy load, vector cache
│   ├── onnx.rs                 ONNX session + inference
│   ├── tokenize.rs             HuggingFace tokenizer wrapper
│   └── cosine.rs               Brute-force cosine similarity search
└── download.rs                 HuggingFace model downloader
```

---

## Build options

```bash
# Full build with ONNX embeddings
cargo build --release --features embedding

# Lightweight build (FTS5-only, no AI model needed)
cargo build --release

# Run tests
cargo test
```

## Requirements

- macOS (Apple Silicon M1+). Linux/Intel x86_64 should work with onnxruntime installed.
- Rust 1.85+
- ONNX Runtime (`brew install onnxruntime`) — optional, only needed with `--features embedding`
- ~2 GB free disk (models ~560 MB + database)

## License

MIT
