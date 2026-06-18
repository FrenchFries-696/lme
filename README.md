# lme — Local Memory Engine

Give Claude long-term memory. Claude remembers your projects across sessions.

No cloud. Everything runs on your machine.

---

## Setup (3 steps)

### Step 1 — Install ONNX Runtime

Open Terminal. Copy and paste:

```bash
brew install onnxruntime
```

One-time setup. Installs the AI model engine.

### Step 2 — Build + download models

```bash
cd /path/to/2ndbrain
cargo build --release --features embedding
cargo run --release --features embedding -- download-models
```

First line compiles the engine. Second downloads AI models (~560 MB). One-time setup.

### Step 3 — Connect to Claude Code

**First, find your project path.** Run `pwd` in the 2ndbrain folder. Copy the output.

Open `~/.claude/mcp.json`. If it doesn't exist, create it. Paste this — **replace the paths with your own**:

```json
{
  "mcpServers": {
    "lme": {
      "command": "/your/path/to/2ndbrain/target/release/lme",
      "env": {
        "LME_CONFIG": "/your/path/to/2ndbrain/lme.toml"
      }
    }
  }
}
```

Example — if you cloned to `~/Projects/2ndbrain`:

```json
{
  "mcpServers": {
    "lme": {
      "command": "/Users/yourname/Projects/2ndbrain/target/release/lme",
      "env": {
        "LME_CONFIG": "/Users/yourname/Projects/2ndbrain/lme.toml"
      }
    }
  }
}
```

Save. Quit Claude Code completely. Reopen.

Done. Claude now has memory.

---

## Verify it works

Start a Claude Code session. Ask:

> "Use lme_status to check the memory engine"

Claude replies with engine stats: total memories, database size, model loaded.

---

## Config

Edit `lme.toml` in the project folder:

```toml
[user]
owner_id = "your-name"                    # Change this

[decay]
lambda_conversation = 0.01                # Forget chats fast
lambda_architecture = 0.001               # Keep architecture slow

[limits]
max_context_chars = 50000                 # Context Claude gets per session
```

Restart Claude Code after changes.

---

## What Claude can do

Claude uses these automatically. You don't call them yourself.

| Tool | What it does |
|------|--------------|
| `lme_store` | Save to memory |
| `lme_context` | Load project memory |
| `lme_search` | Search memories |
| `lme_recall` | Pull up specific memory |
| `lme_status` | Check engine health |

---

## Troubleshooting

**"config file not found"**
→ `LME_CONFIG` path in `mcp.json` is wrong. Make sure it matches your `pwd` output.

**"embed_backend: not-loaded"**
→ Models not downloaded. Run Step 2 again.

**Claude doesn't see the tools**
→ Quit Claude Code and reopen. MCP only loads at startup.

**Build fails**
→ Did `brew install onnxruntime` succeed? (Step 1). Re-run it if unsure.
