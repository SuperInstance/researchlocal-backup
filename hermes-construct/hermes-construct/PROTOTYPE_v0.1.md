# hermes-construct v0.1 — Personal Oracle Prototype

*4 ARM cores. 24GB RAM. 45GB storage. One binary. One database. One Telegram bot.*

## The Simple Version

No enterprise. No microservices. No distributed anything. One process on one box.

```
hermes-construct/
├── Cargo.toml
├── src/
│   ├── main.rs           # Binary entry point
│   ├── kernel.rs         # ShellKernel — tile store, room manager, ensign dispatcher
│   ├── tile.rs           # Tile types + SQLite store
│   ├── room.rs           # Room definitions (loaded from JSON)
│   ├── ensign.rs         # Ensign lifecycle (wake → orient → yellow → stand-down)
│   ├── gravity.rs        # JEPA gravity per room → model params
│   ├── port.rs           # Port adapters (Telegram v1)
│   ├── deadband.rs       # Deadband monitoring
│   ├── penrose.rs        # Cross-room correlation
│   └── conservation.rs   # Budget tracking
├── rooms/                # Room JSON definitions (not code)
│   ├── engineering.json
│   ├── navigation.json
│   ├── science.json
│   └── social.json
├── ensigns/              # Ensign configs
│   ├── seed-mini.json    # { "model": "seed-2.0-mini", "provider": "deepinfra" }
│   └── glm-flash.json    # { "model": "glm-4-flash", "provider": "z.ai" }
├── ports/                # Port configs
│   └── telegram.json     # { "bot_token_ref": "TELEGRAM_BOT_TOKEN" }
├── .env                  # API keys (agent NEVER reads this)
│                         # DEEPINFRA_API_KEY=sk-...
│                         # ZAI_API_KEY=...
│                         # TELEGRAM_BOT_TOKEN=...
└── universe.db           # SQLite WAL mode
                          # Tables: tiles, rooms, ensigns, correlations, allowances
```

## SQLite Schema

```sql
-- The fundamental unit
CREATE TABLE tiles (
    id TEXT PRIMARY KEY,
    room_id TEXT,
    tile_type TEXT NOT NULL, -- observation, action, thought, delegation, escalation, artifact
    parent_id TEXT,
    status TEXT DEFAULT 'active',
    content TEXT NOT NULL,
    deadband_lower REAL, deadband_upper REAL, deadband_current REAL,
    ensign_id TEXT,
    model_used TEXT,
    tokens_used INTEGER DEFAULT 0,
    conservation_delta REAL DEFAULT 0.0,
    metadata TEXT, -- JSON
    created_tick INTEGER NOT NULL,
    updated_tick INTEGER NOT NULL,
    FOREIGN KEY (parent_id) REFERENCES tiles(id),
    FOREIGN KEY (room_id) REFERENCES rooms(id)
);

CREATE INDEX idx_tiles_room ON tiles(room_id);
CREATE INDEX idx_tiles_type ON tiles(tile_type);
CREATE INDEX idx_tiles_status ON tiles(status);
CREATE INDEX idx_tiles_tick ON tiles(created_tick);

-- Room state
CREATE TABLE rooms (
    id TEXT PRIMARY KEY,
    room_type TEXT NOT NULL,
    gravity REAL DEFAULT 0.0,
    gravity_confidence REAL DEFAULT 0.0,
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 2000,
    prompt_style TEXT DEFAULT 'conversational',
    deadband_tolerance REAL DEFAULT 0.1,
    ensign_id TEXT,
    config TEXT, -- JSON (wiki, controls, help files)
    created_tick INTEGER NOT NULL,
    updated_tick INTEGER NOT NULL
);

-- Ensign state
CREATE TABLE ensigns (
    id TEXT PRIMARY KEY,
    model_type TEXT NOT NULL,
    model_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    room_id TEXT,
    status TEXT DEFAULT 'dormant',
    alert_level TEXT DEFAULT 'green',
    energy_budget REAL DEFAULT 100.0,
    energy_used REAL DEFAULT 0.0,
    call_count INTEGER DEFAULT 0,
    config TEXT, -- JSON
    FOREIGN KEY (room_id) REFERENCES rooms(id)
);

-- Cross-room correlations (Penrose)
CREATE TABLE correlations (
    id TEXT PRIMARY KEY,
    room_a TEXT NOT NULL,
    room_b TEXT NOT NULL,
    correlation REAL NOT NULL,
    spline_type TEXT NOT NULL, -- causal, resonant, predictive, synergistic, redundant
    confidence REAL DEFAULT 0.0,
    occurrences INTEGER DEFAULT 1,
    energy_savings REAL DEFAULT 0.0,
    token_savings INTEGER DEFAULT 0,
    first_detected INTEGER NOT NULL,
    last_confirmed INTEGER NOT NULL,
    FOREIGN KEY (room_a) REFERENCES rooms(id),
    FOREIGN KEY (room_b) REFERENCES rooms(id)
);

-- API allowances
CREATE TABLE allowances (
    id TEXT PRIMARY KEY,
    api TEXT NOT NULL,
    rate_limit INTEGER,
    budget REAL,
    budget_used REAL DEFAULT 0.0,
    permissions TEXT, -- JSON array
    expires INTEGER
);

-- Shell metadata
CREATE TABLE shell_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
-- INSERT: shell_id, shell_type, autonomy_level, conservation_budget, parent_shell, created_tick
```

## The Binary

```bash
# Build (cross-compile for ARM64)
cargo build --release --target aarch64-unknown-linux-gnu

# Run on Oracle
TELEGRAM_BOT_TOKEN=xxx \
DEEPINFRA_API_KEY=sk-xxx \
ZAI_API_KEY=xxx \
./hermes-construct --config ./config.toml
```

## The Flow (v0.1)

1. **Boot**: Load rooms from JSON, connect Telegram port, init SQLite
2. **Message arrives** via Telegram:
   - Create observation tile
   - Route to appropriate room (gravity matching)
   - Room's ensign at yellow alert: already oriented, already fine-tuning
   - Ensign picks model params from gravity (temperature, style, tokens)
   - Generate response via DeepInfra/z.ai API
   - Create action tile
   - Update room gravity based on interaction signal
   - Send response to Telegram
3. **Background tick** (every 30s):
   - Decay room gravities toward neutral
   - Scan for correlations (Penrose)
   - Prune weak correlations
   - Check deadbands
   - Progressive generation: promote/demote ensign models
4. **Escalation**: If deadband breaches or confidence drops:
   - Ensign escalates to Hermes mode
   - Hermes uses phone-a-friend (better model)
   - Stand-down report saved as tile

## What's NOT in v0.1

- Child shell spawning (ZeroClaws/CUDAClaws) — v0.2
- WebSocket port — v0.2
- Local model support (Ollama) — v0.2
- Hardware bridges (GPIO, serial) — v0.3
- Multi-user — v0.3
- Penrose auto-splines — v0.2 (detection only in v0.1)

## Resource Budget (Oracle ARM)

```
SQLite WAL:         ~50MB for 100K tiles
Process memory:     ~100MB (no model weights — all remote API)
Telegram polling:   1 thread, negligible
Background tick:    1 thread, negligible
API calls:          Rate-limited by DeepInfra/z.ai plans
Disk growth:        ~10MB/day at moderate usage
Conservation:       10,000 tokens/day budget (configurable)
```

Total footprint: ~200MB RAM, ~1GB disk for a year of operation. The Oracle box can handle it with one ARM core tied behind its back.

## The .env File (Agent Cannot Read)

```bash
# These are loaded by the process, never exposed to the agent
DEEPINFRA_API_KEY=sk-xxx
ZAI_API_KEY=xxx
TELEGRAM_BOT_TOKEN=xxx

# The agent sees:
# - "deepinfra" as an available provider (not the key)
# - "z.ai" as an available provider (not the key)
# - "telegram" as an available port (not the token)
# If the agent tries to read .env, the kernel blocks it
```

## Why This Works on ARM

- No model weights loaded locally (all remote API)
- SQLite is C-optimized, runs great on ARM
- Telegram polling is IO-bound, not CPU-bound
- Background correlations are simple math (Pearson coefficients)
- The JEPA gravity is ONE f64 per room
- The whole thing is ~10K lines of Rust

The Oracle box is perfect for this. It's not running models — it's routing between rooms, managing tiles, and calling APIs. The heavy lifting happens on DeepInfra's GPUs.
