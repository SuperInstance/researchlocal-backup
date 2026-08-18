# Hermes Construct — Quickstart Guide

Get a tile-operating shell running in 5 minutes. No external APIs needed for the basics.

## Prerequisites

- Rust 1.75+ (`rustup update stable`)
- A Telegram Bot Token (optional — stdio works without it)
- API keys for DeepInfra or z.ai (optional — examples use mock providers)

## Step 1: Clone

```bash
git clone https://github.com/SuperInstance/hermes-construct.git
cd hermes-construct
```

## Step 2: Configure

```bash
cp .env.example .env
# Edit .env — add your API keys (or leave empty for stdio mode)
```

Minimum viable `.env`:

```env
# Leave empty to use stdio port instead of Telegram
TELEGRAM_BOT_TOKEN=

# Optional: enables API-powered completions
DEEPINFRA_API_KEY=
ZAI_API_KEY=

# Logging verbosity
RUST_LOG=info
```

## Step 3: Build

```bash
cargo build --release
```

First build takes 2-5 minutes (compiles SQLite, tokio, teloxide).

## Step 4: Run the Examples (No API Keys Needed)

Each example demonstrates a subsystem without external dependencies:

```bash
# Full message flow with mock provider
cargo run --example basic_agent

# Shell spawning and sandboxing
cargo run --example sandbox_demo

# Cross-room correlation detection
cargo run --example correlation_demo

# Provenance chain-of-custody
cargo run --example provenance_demo

# Deadband circuit monitoring
cargo run --example circuit_demo
```

## Step 5: Run the Main Binary

```bash
# Without Telegram (stdio mode)
cargo run --release

# With Telegram
TELEGRAM_BOT_TOKEN=your-token cargo run --release
```

## What You Should See (Bootstrap Output)

```
[INFO] hermes-construct v0.1 starting...
[INFO] Loaded 3 rooms from rooms/
[INFO] Loaded 3 ensigns from ensigns/
[INFO] DeepInfra provider registered    # (if key is set)
[INFO] z.ai provider registered         # (if key is set)
[INFO] Starting Telegram polling...     # (if token is set)
[INFO] Hermes Construct v0.1 running. Ctrl+C to stop.
```

## Step 6: Send Your First Message

**Via Telegram:** Just message your bot directly.

**Via Stdio:** The kernel uses `StdioPort` when no Telegram token is set. Push messages programmatically:

```rust
let port = StdioPort::new();
port.push_message(PortMessage {
    id: uuid::Uuid::new_v4().to_string(),
    text: "Build me a sensor array".into(),
    chat_id: 0,
    from_user: Some("you".into()),
    timestamp: 0,
}).await;
```

## What Happens Inside

When you send "Build me a sensor array":

```
1. Port receives message → PortMessage { text: "Build me a sensor array", ... }
2. Kernel creates Observation tile (cost: 0.1 energy)
3. Room router matches "build" → engineering room (gravity: -0.3)
4. Gravity maps to params: temp=0.3, style="precise", max_tokens=500
5. Ensign ensign-eng activated (model: seed-2.0-mini)
6. Provider called with prompt + system prompt
7. Response received → Action tile created (cost: 0.6 energy)
8. Room gravity nudged (+0.05 for successful response)
9. Tiles persisted to SQLite
10. Response sent back through port
```

## Step 7: Check Status

The conservation budget tracks everything:

```sql
-- Open universe.db after a run
sqlite3 universe.db "SELECT * FROM conservation"
sqlite3 universe.db "SELECT tile_type, COUNT(*) FROM tiles GROUP BY tile_type"
sqlite3 universe.db "SELECT room_a, room_b, correlation, spline_type FROM correlations"
```

## Step 8: Spawning Your First ZeroClaw

ZeroClaws are sandboxed sub-agents. Each gets its own "universe" (SQLite DB).

```bash
# See it in action:
cargo run --example sandbox_demo
```

Key properties:
- Each ZeroClaw has its own isolated SQLite connection
- Cannot access parent universe data
- Conservation budget is tracked separately
- Destroyed automatically when their task is done

## Step 9: Understanding the Room System

Rooms are the organizational unit. Each has:
- **Gravity** (-1.0 to +1.0): Maps to model parameters (precision ↔ creativity)
- **Ensign**: The model assigned to handle messages
- **Deadband tolerance**: How much drift is acceptable
- **Wiki pages**: Context for the ensign
- **Controls**: Available actions

Create rooms by adding JSON files to `rooms/`:

```bash
cp templates/rooms/engineering.json rooms/my-room.json
# Edit and restart
```

## Next Steps

- Read [ARCHITECTURE.md](./ARCHITECTURE.md) for the full system design
- Explore `templates/` for room, ensign, and port configurations
- Run the examples to understand each subsystem
- Build your own rooms and ensigns
- Deploy on Oracle ARM for production

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Build fails on SQLite | Install `libsqlite3-dev` or use `bundled` feature (already configured) |
| No response from bot | Check `TELEGRAM_BOT_TOKEN` is correct, bot is started via BotFather |
| "conservation budget exceeded" | Set `HERMES_CONSERVATION_BUDGET` higher in `.env` |
| Empty rooms directory | Copy templates: `cp -r templates/rooms/ rooms/` and `cp -r templates/ensigns/ ensigns/` |
| No ensigns loaded | Create ensign configs in `ensigns/` directory |
