# Hermes Construct — Architecture

## System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        hermes-construct                          │
│                     (binary entry point)                         │
│                                                                  │
│  main.rs                                                        │
│    ├── dotenvy          (.env loading — API keys NEVER reach     │
│    │                     agent logic, only Provider structs)     │
│    ├── ShellKernel::bootstrap()                                  │
│    │     ├── init SQLite schemas (6 tables)                      │
│    │     ├── load rooms/  (JSON → SQLite)                        │
│    │     └── load ensigns/ (JSON → SQLite)                       │
│    ├── register Providers (DeepInfra, z.ai)                      │
│    ├── register Ports (Telegram, Stdio)                          │
│    └── kernel.run()                                              │
│          ├── poll ports for messages                             │
│          └── background tick (30s)                               │
│                ├── decay room gravities                          │
│                ├── scan correlations (Penrose)                   │
│                └── check deadband circuits                       │
└──────────────────────────────────────────────────────────────────┘
```

## Source Module Map

```
hermes-construct/src/
│
├── main.rs              Binary entry, .env loading, provider/port setup
│
├── kernel.rs            ShellKernel — the main tick loop
│   ├── bootstrap()      Init all schemas, load rooms & ensigns
│   ├── process_message() Full message pipeline (10 steps)
│   ├── background_tick() Periodic: gravity decay, correlations, deadbands
│   └── run()            Main event loop (tokio::select!)
│
├── conservation.rs      Energy budget tracking
│   ├── ConservationState  { budget, used, tick }
│   ├── costs::*          Cost constants for every operation
│   ├── spend()           Deduct from budget (enforces limit)
│   └── save_state()      Persist to SQLite
│
├── gravity.rs           Gravity → model parameter mapping
│   ├── ModelParams       { temperature, prompt_style, max_tokens, top_p, ... }
│   ├── gravity_to_params()  Maps -1.0..+1.0 → algorithmic params
│   └── style_to_system_prompt()  Converts style to system prompt
│
├── room.rs              Room management and routing
│   ├── Room              { id, room_type, gravity, deadband_tolerance, ... }
│   ├── load_rooms_from_dir()  JSON configs → SQLite
│   ├── route_to_room()    Keyword-based message routing
│   ├── nudge_gravity()    Adjust gravity on interaction
│   └── decay_gravity()    Periodic gravity decay toward 0.0
│
├── ensign.rs            Ensign lifecycle + provider abstraction
│   ├── Ensign            { id, model_name, provider, status, ... }
│   ├── EnsignStatus      Dormant→Waking→Orienting→YellowAlert→StandingDown
│   ├── Provider trait    async complete() → CompletionResponse
│   ├── DeepInfraProvider  HTTP client for DeepInfra API
│   ├── ZaiProvider        HTTP client for z.ai API
│   └── load_ensigns_from_dir()  JSON configs → SQLite
│
├── tile.rs              Tile types and SQLite CRUD
│   ├── Tile              { id, room_id, tile_type, parent_id, status, ... }
│   ├── TileType          Observation, Action, Thought, Delegation, Escalation, Artifact
│   ├── TileStatus        Active, Complete, Deadband, Escalated, Archived
│   ├── insert_tile()     Persist to SQLite
│   └── query_tiles()     Filter by room, type, status
│
├── penrose.rs           Cross-room correlation detection
│   ├── Correlation       { room_a, room_b, coefficient, spline_type, ... }
│   ├── SplineType        Causal, Resonant, Predictive, Synergistic, Redundant
│   ├── pearson()         Pearson correlation coefficient
│   ├── classify_correlation()  Coefficient → spline type
│   └── scan_correlations()  All-pairs scan on gravity histories
│
├── deadband.rs          Deadband monitoring and trend detection
│   ├── DeadbandCircuit   { setpoint, tolerance, action, last_value, ... }
│   ├── Trend             Stable, Drifting, Oscillating, Diverging
│   ├── check()           Evaluate current value against bounds
│   ├── detect_trend()    Analyze recent values for trend
│   └── run_checks()      Evaluate all circuits
│
└── port.rs              Communication ports
    ├── Port trait        receive() + send() + is_active()
    ├── TelegramPort      Teloxide-based Telegram adapter
    └── StdioPort         Stdin/stdout for local testing
```

## Data Flow Diagram

```
External                  Ports                     Kernel                      Storage
───────                  ─────                     ──────                      ───────

 Telegram ──────► TelegramPort ──► process_message() ──────► SQLite (tiles)
                     │                    │                          (rooms)
 User message         │                    ├─ 1. Create obs tile     (ensigns)
                     │                    ├─ 2. Route to room       (conservation)
                     │                    ├─ 3. Get ensign          (correlations)
                     │                    ├─ 4. Map gravity→params  (deadband_circuits)
                     │                    ├─ 5. Find provider
                     │                    ├─ 6. Call API ─────────────────────► DeepInfra / z.ai
                     │                    ├─ 7. Create action tile
                     │                    ├─ 8. Update room gravity
                     │                    ├─ 9. Persist tiles
 Stdio    ──────► StdioPort    ──►        └─10. Send response ◄──────────────── Telegram / Stdio
                                        │
                           background_tick() (every 30s)
                                        ├─ Decay gravities
                                        ├─ Scan correlations (Penrose)
                                        └─ Check deadbands
```

## Gravity → Model Parameters

```
  -1.0           -0.5            0.0            +0.5           +1.0
   │               │              │               │              │
   ▼               ▼              ▼               ▼              ▼
 precise        balanced       balanced       creative      narrative
 temp=0.3       temp=0.5       temp=0.5       temp=0.7      temp=0.9
 tokens=500     tokens=1000    tokens=1000    tokens=2000   tokens=4000
 top_p=0.9      top_p=0.95     top_p=0.95     top_p=0.95    top_p=0.95

   ◄─── engineering ────►  ◄─── navigation ───►  ◄── social ──►
        (gravity=-0.3)          (gravity=0.1)       (gravity=0.5)
```

## SQLite Schema Relationships

```
rooms ──────────────┐
  │                  │
  ├── ensigns        │  (each room has an assigned ensign)
  │                  │
  ├── tiles          │  (all tiles belong to a room)
  │   │              │
  │   └── tiles      │  (parent_id → self-referential for chains)
  │                  │
  ├── correlations ──┤  (room_a, room_b → rooms)
  │                  │
  └── deadband_circuits  (room_id → rooms)

conservation         (global budget tracking, key-value)
shell_meta           (shell metadata, key-value)
```

## Crate Dependencies

```
hermes-construct
├── tokio (async runtime, features: full)
├── rusqlite (SQLite, features: bundled)
├── serde + serde_json (serialization)
├── reqwest (HTTP client, features: json)
├── teloxide (Telegram Bot API, features: macros)
├── dotenvy (.env loading)
├── uuid (unique IDs, features: v4)
├── chrono (timestamps, features: serde)
├── log + env_logger (logging)
├── thiserror (error types)
└── async-trait (async trait support)
```

## Conservation Cost Table

Every operation has an energy cost. The budget enforces total spending.

| Operation | Cost (energy) | When |
|-----------|--------------|------|
| Tile create | 0.1 | Every new tile |
| Tile complete | 0.05 | Marking done |
| Ensign activate | 1.0 | Waking an ensign |
| Ensign orient | 0.5 | Reading room context |
| Ensign tile | 0.5 | Processing a tile |
| Ensign stand down | 0.3 | Going dormant |
| Gravity update | 0.01 | Each gravity nudge |
| Phone a friend | 5.0 | Escalation to human |
| Correlation compute | 0.05 | Each pair scan |
| Deadband check | 0.02 | Each circuit check |
| Shell spawn | 5.0 | Creating a ZeroClaw |
| Shell destroy | 2.0 | Destroying a ZeroClaw |
| Bootstrap step | 0.5 | Each bootstrap phase |
| Port open/close | 0.2 | Starting/stopping port |
| Port message | 0.01 | Each message through port |

## Ensign Lifecycle

```
Dormant ──wake()──► Waking ──orient()──► Orienting
                                          │
                              ┌───────────┤
                              ▼           ▼
                         GreenAlert  YellowAlert
                              │           │
                              │    (can handle)
                              │           │
                              ▼           ▼
                         StandingDown ◄───┘
                              │
                              ▼
                           Dormant
```

Alert levels: Green (normal) → Yellow (active) → Red (escalated)

## Key Design Principles

1. **The tile is the fundamental unit of work** — Everything is a tile. Observations, actions, thoughts, delegations, escalations, artifacts.

2. **The room is the agent** — Each room has its own gravity, ensign, wiki, and controls. Rooms are the organizational boundary.

3. **Conservation is enforced** — Every operation costs energy. You can't spend what you don't have.

4. **API keys never reach agent logic** — Loaded in main.rs, wrapped in Provider structs. The ensign sees only the Provider trait.

5. **Ports are pluggable** — Telegram, stdio, or any future transport. The kernel doesn't care which port a message came from.

6. **Correlations are detected, not declared** — Penrose scans gravity histories and discovers relationships between rooms automatically.

7. **Deadbands prevent silent failures** — Circuits monitor values and alert before things go wrong.
