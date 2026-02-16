# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
# Build
cargo build                     # Debug build (default-members = cli)
cargo build --release           # Release build (~27MB binary)
cargo build --workspace         # Build all crates

# Run
cargo run -- <subcommand>       # Run with arguments
cargo run -- chat               # Interactive chat
cargo run -- ask "question"     # Single question
cargo run -- daemon start       # Start daemon with HTTP server

# Test
cargo test --workspace          # Run all tests across all crates
cargo test -p localgpt-core     # Test a specific crate
cargo test -- --nocapture       # Show test output

# Lint
cargo clippy --workspace
cargo fmt --check

# Cross-compile checks (mobile targets)
cargo check -p localgpt-core --target aarch64-apple-ios
cargo check -p localgpt-mobile --target aarch64-apple-ios
cargo check -p localgpt-core --target aarch64-apple-ios-sim

# Headless build (no desktop GUI)
cargo build -p localgpt-cli --no-default-features
```

## Architecture

LocalGPT is a local-only AI assistant with persistent markdown-based memory and optional autonomous operation via heartbeat.

The project is a Rust workspace with 6 crates:

```
crates/
├── core/      # localgpt-core — shared library (agent, memory, config, security)
├── cli/       # localgpt-cli — binary with clap CLI, desktop GUI, dangerous tools
├── server/    # localgpt-server — HTTP/WebSocket API, Telegram bot, optional WASM web UI
├── sandbox/   # localgpt-sandbox — Landlock/Seatbelt process sandboxing
├── mobile/    # localgpt-mobile — UniFFI bindings for iOS/Android
└── gen/       # localgpt-gen — Bevy 3D scene generation binary

mobile/        # Native app projects (not Rust)
├── ios/       # iOS build scripts (build-rust.sh → XCFramework)
└── android/   # Android build config (cargo-ndk via rust.gradle)
```

### Crate Dependency Graph

```
localgpt-cli ──→ localgpt-core
             ──→ localgpt-server ──→ localgpt-core
             ──→ localgpt-sandbox ──→ localgpt-core

localgpt-mobile ──→ localgpt-core (no default features, embeddings-openai only)
localgpt-gen    ──→ localgpt-core
```

**Rule:** `localgpt-core` has zero platform-specific dependencies. No clap, rustyline, daemonize, eframe, axum, teloxide, landlock, seccompiler, nix. It compiles cleanly for `aarch64-apple-ios` and `aarch64-linux-android`.

### Core (`crates/core/` — `localgpt-core`)

Platform-independent library. Compiles for iOS/Android targets.

- **agent/** - LLM interaction layer
  - `providers.rs` - Trait `LLMProvider` with implementations for OpenAI, Anthropic, Ollama, Claude CLI (feature-gated: `claude-cli`), and GLM (Z.AI). Model prefix determines provider (`claude-cli/*` → Claude CLI, `gpt-*` → OpenAI, `claude-*` → Anthropic API, `glm-*` → GLM, else Ollama)
  - `session.rs` - Conversation state with automatic compaction when approaching context window limits
  - `session_store.rs` - Session metadata store (`sessions.json`) with CLI session ID persistence
  - `system_prompt.rs` - Builds system prompt with identity, safety, workspace info, tools, skills, and special tokens
  - `skills.rs` - Loads SKILL.md files from workspace/skills/ for specialized task handling
  - `tools/mod.rs` - Safe tools only: `memory_search`, `memory_get`, `web_fetch`, `web_search`
  - `AgentHandle` - Thread-safe `Arc<tokio::sync::Mutex<Agent>>` wrapper for mobile/server use
- **memory/** - Markdown-based knowledge store (SQLite FTS5, file watcher, workspace templates)
  - Feature-gated embeddings: `embeddings-local` (fastembed/ONNX, default), `embeddings-openai`, `embeddings-gguf`, `embeddings-none`
  - Feature-gated provider: `claude-cli` (default) — subprocess-based Claude CLI; excluded on mobile
- **heartbeat/** - Autonomous task runner on configurable interval
- **config/** - TOML configuration at `~/.localgpt/config.toml`. `Config::load()` for desktop, `Config::load_from_dir()` for mobile
- **commands.rs** - Shared slash command definitions used by CLI and Telegram
- **concurrency/** - TurnGate and WorkspaceLock
- **paths.rs** - XDG directory resolution. `Paths::resolve()` for desktop, `Paths::from_root()` for mobile
- **security/** - LocalGPT.md policy signing/verification

### CLI (`crates/cli/` — `localgpt-cli`)

Binary crate (`localgpt` binary). Adds dangerous tools (bash, read_file, write_file, edit_file) via `tools.rs` and `agent.extend_tools()`.

- **cli/** - Clap subcommands: chat, ask, daemon, memory, config, md, sandbox, search, paths
- **tools.rs** - CLI-only tools with sandbox integration (`create_cli_tools()`)
- **desktop/** - Optional eframe/egui desktop GUI (feature `desktop`)

### Server (`crates/server/` — `localgpt-server`)

- **http.rs** - Axum REST API with embedded Web UI. Endpoints: `/health`, `/api/status`, `/api/chat`, `/api/memory/search`, `/api/memory/stats`
- **telegram.rs** - Telegram bot with pairing auth, streaming, per-chat sessions
- **websocket.rs** - WebSocket support
- Optional feature `egui-web` for WASM-based web UI

### Sandbox (`crates/sandbox/` — `localgpt-sandbox`)

- Process sandboxing: Landlock (Linux), Seatbelt (macOS)
- Policy builder, capability detection, sandbox child exec

### Mobile (`crates/mobile/` — `localgpt-mobile`)

UniFFI proc-macro bindings for iOS (Swift) and Android (Kotlin).

- `LocalGPTClient` - Thread-safe client object with its own tokio runtime
- Exposes: `chat`, `memory_search`, `memory_get`, `get_soul`/`set_soul`, `get_model`/`set_model`, `session_status`, `new_session`, `compact_session`, `configure_provider`, `list_providers`
- Uses `localgpt-core` with `default-features = false, features = ["embeddings-openai"]`
- Error type: `MobileError` enum (Init, Chat, Memory, Config)

### Gen (`crates/gen/` — `localgpt-gen`)

Standalone binary for AI-driven 3D scene generation via Bevy.

- Bevy runs on main thread (macOS windowing/GPU requirement), agent loop on background tokio runtime
- Uses `Agent::new_with_tools()` with custom Gen tools (spawn_entity, modify_entity, etc.)

### Key Patterns

- `Agent::new()` creates safe tools only; CLI extends with `agent.extend_tools(create_cli_tools())`
- `Agent::new_with_tools()` for fully custom tool sets (Gen mode)
- `AgentHandle` wraps Agent in `Arc<tokio::sync::Mutex>` for thread-safe access from mobile/server
- Agent is not `Send+Sync` due to SQLite connections — HTTP handler uses `spawn_blocking`, mobile uses `AgentHandle`
- Session compaction triggers memory flush (prompts LLM to save important context before truncating)
- Memory context automatically loaded into new sessions: `MEMORY.md`, recent daily logs, `HEARTBEAT.md`
- Tools use `shellexpand::tilde()` for path expansion

## Configuration

Default config path: `~/.localgpt/config.toml` (see `config.example.toml`)

**Auto-creation**: If no config file exists on first run, LocalGPT automatically creates a default config with helpful comments. If an OpenClaw config exists at `~/.openclaw/config.json5`, it will be migrated automatically.

Key settings:
- `agent.default_model` - Model name (determines provider). Default: `claude-cli/opus`. Supported: Anthropic (`anthropic/claude-*`), OpenAI (`openai/gpt-*`), GLM/Z.AI (`glm/glm-4.7` or alias `glm`), Claude CLI (`claude-cli/*`), Ollama (`ollama/*`)
- `agent.context_window` / `reserve_tokens` - Context management
- `memory.workspace` - Workspace directory path. Default: `~/.localgpt/workspace`
- `memory.embedding_provider` - `"local"` (default, fastembed), `"openai"`, or `"none"`
- `heartbeat.interval` - Duration string (e.g., "30m", "1h")
- `heartbeat.active_hours` - Optional `{start, end}` in "HH:MM" format
- `server.port` - HTTP server port (default: 31327)
- `telegram.enabled` - Enable Telegram bot (default: false)
- `telegram.api_token` - Telegram Bot API token (supports `${TELEGRAM_BOT_TOKEN}`)

### Telegram Bot

The Telegram bot runs as a background task inside the daemon (`localgpt daemon start`). It provides the same chat capabilities as the CLI, including tool use and memory access.

**Setup:**

1. Create a bot via [@BotFather](https://t.me/BotFather) and get the API token
2. Configure in `~/.localgpt/config.toml`:
   ```toml
   [telegram]
   enabled = true
   api_token = "${TELEGRAM_BOT_TOKEN}"
   ```
3. Start the daemon: `localgpt daemon start`
4. Message your bot on Telegram — it will generate a 6-digit pairing code printed to the daemon console/logs
5. Send the code back to the bot to complete pairing

**Pairing & Auth:**
- First message triggers a one-time pairing flow with a 6-digit code
- Code is printed to stdout and daemon logs
- Only the paired user can interact with the bot
- Pairing persists in `~/.localgpt/telegram_paired_user.json`

**Telegram Commands:**
- `/help` - Show available commands
- `/new` - Start fresh session
- `/skills` - List available skills
- `/model [name]` - Show or switch model
- `/compact` - Compact session history
- `/clear` - Clear session history
- `/memory <query>` - Search memory files
- `/status` - Show session info
- `/unpair` - Remove pairing (re-enables pairing flow)

**Implementation notes:**
- Uses agent ID `"telegram"` (separate sessions from CLI's `"main"`)
- Messages >4096 chars are split into chunks for Telegram's limit
- Streaming responses are shown via debounced message edits (every 2s)
- Tool calls are displayed with extracted details during streaming
- Shares `TurnGate` concurrency control with HTTP server and heartbeat

### Workspace Path Customization (OpenClaw-Compatible)

Workspace path resolution order:
1. `LOCALGPT_WORKSPACE` env var - absolute path override
2. `LOCALGPT_PROFILE` env var - creates `~/.localgpt/workspace-{profile}`
3. `memory.workspace` from config file
4. Default: `~/.localgpt/workspace`

Examples:
```bash
# Use a custom workspace directory
export LOCALGPT_WORKSPACE=~/my-project/ai-workspace
localgpt chat

# Use profile-based workspaces (like OpenClaw's OPENCLAW_PROFILE)
export LOCALGPT_PROFILE=work    # uses ~/.localgpt/workspace-work
export LOCALGPT_PROFILE=home    # uses ~/.localgpt/workspace-home
```

## Skills System (OpenClaw-Compatible)

Skills are SKILL.md files that provide specialized instructions for specific tasks.

### Skill Sources (Priority Order)

1. **Workspace skills**: `~/.localgpt/workspace/skills/` (highest priority)
2. **Managed skills**: `~/.localgpt/skills/` (user-level, shared across workspaces)

### SKILL.md Format

```yaml
---
name: github-pr
description: "Create and manage GitHub PRs"
user-invocable: true              # Expose as /github-pr command (default: true)
disable-model-invocation: false   # Include in model prompt (default: false)
command-dispatch: tool            # Optional: direct tool dispatch
command-tool: bash                # Tool name for dispatch
metadata:
  openclaw:
    emoji: "🐙"
    always: false                 # Skip eligibility checks
    requires:
      bins: ["gh", "git"]         # Required binaries (all must exist)
      anyBins: ["python", "python3"]  # At least one required
      env: ["GITHUB_TOKEN"]       # Required environment variables
---

# GitHub PR Skill

Instructions for the agent on how to create PRs...
```

### Skill Features

| Feature | Description |
|---------|-------------|
| **Slash commands** | Invoke skills via `/skill-name [args]` |
| **Requirements gating** | Skills blocked if missing binaries/env vars |
| **Model prompt filtering** | `disable-model-invocation: true` hides from model but keeps command |
| **Multiple sources** | Workspace skills override managed skills of same name |
| **Emoji display** | Show emoji in `/skills` list and `/help` |

## CLI Commands (Interactive Chat)

In the `chat` command, these slash commands are available:

- `/help` - Show available commands
- `/quit`, `/exit`, `/q` - Exit chat
- `/new` - Start fresh session (reloads system prompt and memory context)
- `/skills` - List available skills with status
- `/sessions` - List saved sessions
- `/search <query>` - Search across sessions
- `/resume <id>` - Resume a session
- `/model [name]` - Show or switch model
- `/models` - List model prefixes
- `/context` - Show context window usage
- `/compact` - Compact session history (summarize and truncate)
- `/clear` - Clear session history (keeps current context)
- `/memory <query>` - Search memory files
- `/reindex` - Rebuild memory index
- `/save` - Save current session to disk
- `/export [file]` - Export session as markdown
- `/attach <file>` - Attach file to message
- `/attachments` - List pending attachments
- `/status` - Show session info (ID, messages, tokens, compactions)

Plus any skill slash commands (e.g., `/github-pr`, `/commit`) based on installed skills.

## Mobile Development

### Generate Bindings (dev machine)

```bash
# Build the mobile crate (includes uniffi-bindgen binary)
cargo build -p localgpt-mobile

# Generate Swift bindings
target/debug/uniffi-bindgen generate \
  --library target/debug/liblocalgpt_mobile.dylib \
  --language swift --out-dir mobile/ios/Generated

# Generate Kotlin bindings
target/debug/uniffi-bindgen generate \
  --library target/debug/liblocalgpt_mobile.dylib \
  --language kotlin --out-dir mobile/android/Generated
```

### iOS

```bash
# Prerequisites
rustup target add aarch64-apple-ios aarch64-apple-ios-sim

# Build and generate Swift bindings + XCFramework
cd mobile/ios/scripts
./build-rust.sh          # Release build (default)
./build-rust.sh debug    # Debug build

# Output:
#   mobile/ios/LocalGPTCore.xcframework
#   mobile/ios/Generated/*.swift
```

### Android

```bash
# Prerequisites
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk

# Build via gradle (in Android project)
# See mobile/android/gradle/rust.gradle
```

### Swift Usage Example

```swift
import LocalGPTCore

let client = try LocalGPTClient(dataDir: documentsDirectory.path)
let response = try client.chat(message: "Hello!")
let results = try client.memorySearch(query: "projects", maxResults: 5)
```

### Kotlin Usage Example

```kotlin
import app.localgpt.core.LocalGPTClient

val client = LocalGPTClient(dataDir = filesDir.absolutePath)
val response = client.chat(message = "Hello!")
val results = client.memorySearch(query = "projects", maxResults = 5u)
```

## OpenClaw Compatibility

LocalGPT maintains strong compatibility with OpenClaw workspace files for seamless migration.

### Directory Structure

```
~/.localgpt/                          # State directory (OpenClaw: ~/.openclaw/)
├── config.toml                       # Config (OpenClaw uses JSON5)
├── agents/
│   └── main/                         # Default agent ID
│       └── sessions/
│           ├── sessions.json         # Session metadata (compatible format)
│           └── <sessionId>.jsonl     # Session transcripts (Pi format)
├── workspace/                        # Memory workspace (fully compatible)
│   ├── MEMORY.md                     # Long-term curated memory
│   ├── HEARTBEAT.md                  # Pending autonomous tasks
│   ├── SOUL.md                       # Persona and tone guidance
│   ├── USER.md                       # User profile (OpenClaw)
│   ├── IDENTITY.md                   # Agent identity (OpenClaw)
│   ├── TOOLS.md                      # Tool notes (OpenClaw)
│   ├── AGENTS.md                     # Operating instructions (OpenClaw)
│   ├── memory/
│   │   └── YYYY-MM-DD.md             # Daily logs
│   ├── knowledge/                    # Knowledge repository (optional)
│   └── skills/                       # Custom skills
└── logs/
```

### Workspace Files (Full Compatibility)

OpenClaw workspace files are fully supported. Copy them directly:

| File | Purpose | LocalGPT Support |
|------|---------|------------------|
| `MEMORY.md` | Long-term curated knowledge (facts, preferences, decisions) | Full |
| `HEARTBEAT.md` | Pending tasks for autonomous heartbeat runs | Full |
| `SOUL.md` | Persona, tone, and behavioral boundaries | Full |
| `USER.md` | User profile and addressing preferences | Loaded |
| `IDENTITY.md` | Agent name, vibe, emoji | Loaded |
| `TOOLS.md` | Notes about local tools and conventions | Loaded |
| `AGENTS.md` | Operating instructions for the agent | Loaded |
| `memory/*.md` | Daily logs (YYYY-MM-DD.md format) | Full |
| `knowledge/` | Structured knowledge repository | Indexed |
| `skills/*/SKILL.md` | Specialized task instructions | Full |

### Session Format (Pi-Compatible)

Session transcripts use Pi's SessionManager JSONL format for OpenClaw compatibility:

**Header** (first line):
```json
{"type":"session","version":1,"id":"uuid","timestamp":"2026-02-03T10:00:00Z","cwd":"/path","compactionCount":0,"memoryFlushCompactionCount":0}
```

**Messages**:
```json
{"type":"message","message":{"role":"user","content":[{"type":"text","text":"Hello"}],"timestamp":1234567890}}
{"type":"message","message":{"role":"assistant","content":[{"type":"text","text":"Hi!"}],"usage":{"input":10,"output":5,"totalTokens":15},"model":"claude-3-opus","stopReason":"end_turn","timestamp":1234567891}}
```

**Tool calls**:
```json
{"type":"message","message":{"role":"assistant","content":[],"toolCalls":[{"id":"call_1","name":"bash","arguments":"{\"command\":\"ls\"}"}]}}
{"type":"message","message":{"role":"toolResult","content":[{"type":"text","text":"file1.txt\nfile2.txt"}],"toolCallId":"call_1"}}
```

### Migrating from OpenClaw

```bash
# Copy workspace files (fully compatible)
cp -r ~/.openclaw/workspace/* ~/.localgpt/workspace/

# Copy session data
cp -r ~/.openclaw/agents ~/.localgpt/agents

# Memory index will be rebuilt automatically on first run
```

## Git Version Control

LocalGPT auto-creates `.gitignore` files. Recommended version control:

**Version control (workspace/):**
- `MEMORY.md` - Your curated knowledge
- `HEARTBEAT.md` - Pending tasks
- `SOUL.md` - Your persona
- `memory/*.md` - Daily logs
- `skills/` - Custom skills

**Do NOT version control:**
- `agents/*/sessions/*.jsonl` - Conversation transcripts (large)
- `logs/` - Debug logs
- `*.db` - SQLite database files

**Be careful with:**
- `config.toml` - May contain API keys. Use `${ENV_VAR}` syntax instead of raw keys.
