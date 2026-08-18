# PLATO Build Plan — Hermes Construct Refactoring

**Status**: Planning
**Date**: 2026-05-30
**Author**: Generated from analysis of hermes-plato-shell + hermes-construct

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Current State Assessment](#current-state-assessment)
3. [Architecture Overview](#architecture-overview)
4. [Phase 1: Core Tile System](#phase-1-core-tile-system)
5. [Phase 2: Ensign Deployment](#phase-2-ensign-deployment)
6. [Phase 3: JEPA Gravity Integration](#phase-3-jepa-gravity-integration)
7. [Phase 4: Progressive Generation](#phase-4-progressive-generation)
8. [Phase 5: Penrose Correlation](#phase-5-penrose-correlation)
9. [Phase 6: Oracle Deployment](#phase-6-oracle-deployment)
10. [Phase 7: Self-Automation](#phase-7-self-automation)
11. [Cross-Cutting Concerns](#cross-cutting-concerns)
12. [Failure Modes & Mitigations](#failure-modes--mitigations)
13. [Dependency Map](#dependency-map)
14. [Testing Strategy](#testing-strategy)
15. [Glossary](#glossary)

---

## Executive Summary

This plan refactors hermes-construct (the Nous Research Hermes Agent) into a PLATO-native agent system called Hermes. The key transformation: from a monolithic Python agent that processes conversation turns → to a tile-operating, room-native system where small models (Ensigns) maintain persistent rooms, a JEPA gravity field governs response style, and progressive autonomy evolves from Level 1 (Opus does everything) to Level 5 (the system runs itself).

The refactoring is structured as 7 phases, each building on the last, each independently deployable and testable. Phase 1–3 form the foundation; Phase 4–5 add intelligence; Phase 6 is production hardening; Phase 7 is the autonomy journey.

**Critical principle**: Every phase must leave Hermes fully functional. We never break the existing agent — we augment it.

---

## Current State Assessment

### What Exists (hermes-construct)

hermes-construct is a ~700k LOC Python agent (fork of Nous Research's Hermes Agent) with:

| Component | File(s) | Purpose |
|-----------|---------|---------|
| Conversation loop | `run_agent.py` (~4700 LOC), `agent/conversation_loop.py` (~4700 LOC) | Core turn processing: model call → tool dispatch → retry → compression |
| Tool system | `tools/*.py` (~80 tools), `toolsets.py` | Tool discovery, registry, dispatch |
| Subagent delegation | `tools/delegate_tool.py` (~2800 LOC) | ThreadPoolExecutor-based child agents |
| State persistence | `hermes_state.py` (~3500 LOC) | SQLite + FTS5 session storage |
| Cron/scheduler | `cron/jobs.py`, `cron/scheduler.py` | Scheduled automations |
| Gateway | `gateway/platforms/*` (~20 platforms) | Telegram, Discord, Slack, etc. |
| Plugins | `plugins/*` (~20 plugins) | Model providers, memory, observability |
| Skills | `skills/*` (~30 skills) | Domain-specific knowledge bundles |
| CLI | `cli.py` (~11k LOC) | Interactive terminal UI |

### What Exists (hermes-plato-shell)

The PLATO shell overlay provides:
- **SOUL.md**: Riker/First Officer persona with override protocol
- **3 PLATO skills**: ecosystem awareness, hardware bridge, subagent archetypes
- **1 plugin**: PLATO plugin hooks (pre/post tool call, session lifecycle)
- **Config overlay**: PLATO-specific config additions
- **Reference docs**: Crate catalog, conservation primer, deployment guide

### What's Missing (The Gap)

The PLATO shell is a **prompt-layer overlay** — it teaches Hermes *about* PLATO concepts but doesn't **implement** them. Specifically missing:

1. **No tile system** — Hermes operates on conversation turns, not composable tiles
2. **No Ensign hooks** — No small-model integration, no yellow-alert lifecycle
3. **No JEPA gravity** — No per-room scalar that tunes generation params
4. **No progressive generation** — Model selection is static per-session
5. **No Penrose correlation** — No cross-room learning mechanism
6. **No room-native architecture** — The "room" concept exists only in prompts, not in code
7. **No baton passing** — Specialists don't beam in/out with state transfer
8. **No deadband circuits** — Automations run on cron, not within deadbands

---

## Architecture Overview

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CAPTAIN (User)                           │
│              Telegram / CLI / Dashboard / Override               │
└────────────────────────────┬────────────────────────────────────┘
                             │ commands + override phrases
┌────────────────────────────▼────────────────────────────────────┐
│                      HERMES CORE                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              TILE ORCHESTRATOR                            │   │
│  │  • Manages tiles (not monolithic conversations)          │   │
│  │  • Routes to rooms                                       │   │
│  │  • Coordinates baton passing between specialists          │   │
│  │  • Logs every tile operation for audit                   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │NAV ROOM │ │ENG ROOM │ │SCI ROOM │ │SOC ROOM │ │SEC ROOM │  │
│  │         │ │         │ │         │ │         │ │         │  │
│  │ ENSIGN  │ │ ENSIGN  │ │ ENSIGN  │ │ ENSIGN  │ │ ENSIGN  │  │
│  │ (seed)  │ │ (seed)  │ │ (glm)   │ │ (glm)   │ │ (seed)  │  │
│  │         │ │         │ │         │ │         │ │         │  │
│  │ JEPA:   │ │ JEPA:   │ │ JEPA:   │ │ JEPA:   │ │ JEPA:   │  │
│  │ -0.3    │ │ -0.6    │ │  0.0    │ │ +0.5    │ │ -0.8    │  │
│  │         │ │         │ │         │ │         │ │         │  │
│  │DEADBAND │ │DEADBAND │ │DEADBAND │ │DEADBAND │ │DEADBAND │  │
│  │ 0.05    │ │ 0.10    │ │ 0.08    │ │ 0.15    │ │ 0.02    │  │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘  │
│       │           │           │           │           │         │
│  ┌────▼───────────▼───────────▼───────────▼───────────▼────┐   │
│  │              JEPA GRAVITY FIELD (per-room f64)           │   │
│  │  Maps gravity → {temperature, prompt_style, max_tokens}  │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│  ┌────────────────────────▼────────────────────────────────┐   │
│  │         PENROSE CORRELATION ENGINE                       │   │
│  │  Rooms learn from each other via proximity splines       │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│  ┌────────────────────────▼────────────────────────────────┐   │
│  │         PROGRESSIVE GENERATION TRACKER                   │   │
│  │  Level 1→5 promotion/demotion based on success rate      │   │
│  │  Controls: which model, which ensign, phone-a-friend     │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│  ┌────────────────────────▼────────────────────────────────┐   │
│  │         PHONE-A-FRIEND (Opus 4.8)                       │   │
│  │  Escalation for hard problems. Decreases over time.      │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
           │ FFI / HTTP / WebSocket
┌──────────▼──────────────────────────────────────────────────────┐
│              LAU-* CRATES (Rust, via FFI or API)                 │
│  lau-room-native │ lau-ensign │ lau-jepa-gravity               │
│  lau-penrose │ lau-intention │ lau-vibe-field                   │
│  lau-construct │ lau-a2ui │ lau-affordance                     │
│  lau-tminus │ lau-plato-tutor                                   │
└──────────────────────────────────────────────────────────────────┘
```

### Tile Architecture

A **tile** is the fundamental unit of work. Everything is a tile:

```python
@dataclass
class Tile:
    id: str                    # UUID
    room: str                  # Which room owns this tile
    type: TileType             # conversation | automation | analysis | monitoring
    model: str                 # Which model generated this (opus/seed/glm)
    ensign: Optional[str]      # Which ensign handled it
    gravity: float             # JEPA gravity at time of creation
    level: int                 # Progressive generation level (1-5)
    input_tokens: int
    output_tokens: int
    latency_ms: float
    success: Optional[bool]    # None = pending
    quality_score: float       # 0.0-1.0, from feedback or heuristics
    baton: Optional[Baton]     # State passed to next specialist
    created_at: datetime
    completed_at: Optional[datetime]
    parent_tile: Optional[str] # For decomposed tasks
    children: List[str]        # Child tile IDs
    log: List[TileLogEntry]    # Audit trail
```

Tiles are:
- **Logged**: Every tile operation is recorded
- **Composable**: Tiles can spawn child tiles
- **Auditable**: Full provenance from input to output
- **Tiled**: The system operates on tiles, not raw conversation turns

### Room-Native Model

A **room** is a persistent context maintained by an Ensign. The room IS the agent's context:

```python
@dataclass
class Room:
    name: str                  # navigation, engineering, science, social, security
    ensign: EnsignConfig       # Model, alert level, deadband
    gravity: float             # JEPA gravity (-1.0 to +1.0)
    state: RoomState           # Current room state (JSON blob)
    deadband: float            # Tolerance for automation
    tile_history: List[str]    # Recent tile IDs
    baton_holder: Optional[str] # Who currently holds the baton
    last_activity: datetime
    alert_level: AlertLevel    # green | yellow | red
```

Rooms:
- **Persist**: State survives across sessions
- **Have gravity**: JEPA scalar tunes response parameters
- **Have deadbands**: Automations run within tolerance bands
- **Pass batons**: Specialists beam in/out with state transfer
- **Learn**: From other rooms via Penrose correlation

---

## Phase 1: Core Tile System

**Goal**: Transform the monolithic conversation loop into a tile-based architecture where every operation is a logged, composable tile.

**Duration**: 2-3 weeks
**Dependencies**: None (foundation phase)

### Files/Modules to Create

```
hermes-construct/
├── plato/
│   ├── __init__.py
│   ├── tile/
│   │   ├── __init__.py
│   │   ├── types.py              # Tile, TileType, TileLogEntry, Baton dataclasses
│   │   ├── orchestrator.py       # TileOrchestrator — creates, routes, tracks tiles
│   │   ├── store.py              # TileStore — SQLite-backed tile persistence
│   │   └── composer.py           # TileComposer — parent/child tile decomposition
│   ├── room/
│   │   ├── __init__.py
│   │   ├── types.py              # Room, RoomState, AlertLevel dataclasses
│   │   ├── manager.py            # RoomManager — CRUD for rooms, state persistence
│   │   └── baton.py              # BatonPass — specialist handoff with state transfer
│   └── config.py                 # PLATO config loading and validation
├── tools/
│   └── plato_tile_tool.py        # Tool exposure: tile status, tile history
├── plugins/
│   └── plato_tile/
│       ├── __init__.py
│       ├── plugin.yaml
│       └── hooks.py              # pre/post conversation turn → tile creation
└── tests/
    └── plato/
        ├── test_tile_types.py
        ├── test_tile_orchestrator.py
        ├── test_tile_store.py
        ├── test_tile_composer.py
        ├── test_room_manager.py
        ├── test_baton_pass.py
        └── test_plato_config.py
```

### Key Types

```python
# plato/tile/types.py
class TileType(Enum):
    CONVERSATION = "conversation"    # User-facing response
    AUTOMATION = "automation"        # Cron/triggered task
    ANALYSIS = "analysis"            # Background analysis
    MONITORING = "monitoring"        # Deadband monitoring tick
    ESCALATION = "escalation"        # Phone-a-friend call
    ORIENTATION = "orientation"      # Ensign room orientation

class AlertLevel(Enum):
    GREEN = "green"    # Idle, monitoring only
    YELLOW = "yellow"  # Active, ready to respond
    RED = "red"        # Emergency, all hands

@dataclass
class Baton:
    """State passed between specialists during baton handoff."""
    from_room: str
    to_room: str
    context: Dict[str, Any]      # Serialized room state
    intention: Optional[str]      # What the next specialist should do
    energy_budget: float          # Remaining energy from conservation budget
    summary: str                  # Human-readable summary of what happened

@dataclass
class TileLogEntry:
    timestamp: datetime
    event: str                    # "created", "started", "completed", "failed", "escalated"
    model: str
    details: Dict[str, Any]
```

### Integration Points

1. **`run_agent.py` / `agent/conversation_loop.py`**: Wrap each conversation turn in a Tile. The `run_conversation()` method creates a tile at entry, updates it on completion. This is a thin wrapper — the existing loop is unchanged, just instrumented.

2. **`tools/delegate_tool.py`**: Each delegation creates a child tile. The parent tile tracks children. When a subagent completes, its tile result bubbles up.

3. **`cron/jobs.py`**: Each cron execution creates an AUTOMATION tile. Tiles persist across runs, enabling analysis of automation quality over time.

4. **`hermes_state.py`**: The TileStore sits alongside SessionDB. Both use SQLite. TileStore has its own DB file (`plato_tiles.db`) to avoid migration coupling.

### Dependencies on lau-* Crates

- **`lau-room-native`** (57 tests): Room IS the agent, baton passing. We implement the Python-side types that mirror the Rust types. FFI integration can come later; initially the Python types stand alone.
- **`lau-construct`** (83 tests): Matrix Construct for tile composition. The `TileComposer` implements the Construct pattern in Python.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_tile_types` | Dataclass construction, serialization, defaults |
| `test_tile_orchestrator` | Tile creation, routing to rooms, lifecycle management |
| `test_tile_store` | SQLite persistence, query by room/type/time, FTS search |
| `test_tile_composer` | Parent/child decomposition, result bubbling |
| `test_room_manager` | Room CRUD, state persistence, baton assignment |
| `test_baton_pass` | State transfer between rooms, energy budget propagation |
| `test_plato_config` | Config loading, validation, defaults |

**Integration tests**: Wrap 10 real conversation turns in tiles, verify all tiles are persisted, queryable, and compose correctly.

### Edge Cases

- **Tile explosion**: A single user request could generate many tiles. Cap children per parent (default: 10). Log overflow.
- **Orphaned tiles**: Tiles whose parent was interrupted. Garbage collect after 24h.
- **Concurrent tile creation**: Two threads creating tiles for the same room. TileStore uses WAL mode + row-level locking.
- **Tile size**: Large tool outputs bloat tiles. Compress >10KB payloads in the store, store reference in the tile.

---

## Phase 2: Ensign Deployment

**Goal**: Integrate small models (Seed-mini, GLM-flash) as Ensigns that maintain rooms, wake on-call, orient from room state, and stay at yellow alert.

**Duration**: 2-3 weeks
**Dependencies**: Phase 1 (rooms and tiles must exist)

### Files/Modules to Create

```
hermes-construct/
├── plato/
│   ├── ensign/
│   │   ├── __init__.py
│   │   ├── types.py              # EnsignConfig, EnsignState, OnCallReason
│   │   ├── ensign.py             # Ensign — the DJ class, manages a room
│   │   ├── pool.py               # EnsignPool — manages all active ensigns
│   │   ├── orient.py             # Orientation — reads room state, builds context
│   │   ├── alert.py              # AlertManager — green/yellow/red transitions
│   │   └── deadband.py           # DeadbandMonitor — runs within tolerance bands
│   └── ...
├── tools/
│   └── plato_ensign_tool.py      # Tool: ensign status, ensign activate/stand_down
├── plugins/
│   └── plato_ensign/
│       ├── __init__.py
│       ├── plugin.yaml
│       └── hooks.py              # on_session_start → wake ensigns
└── tests/
    └── plato/
        ├── test_ensign_types.py
        ├── test_ensign_lifecycle.py
        ├── test_ensign_pool.py
        ├── test_orientation.py
        ├── test_alert_manager.py
        └── test_deadband.py
```

### Key Types

```python
# plato/ensign/types.py
@dataclass
class EnsignConfig:
    name: str                    # e.g., "seed-mini", "glm-flash"
    model: str                   # Model identifier for provider
    provider: str                # "deepinfra", "z.ai", etc.
    alert_default: AlertLevel    # Default alert when activated
    deadband_tolerance: float    # 0.0-1.0, room-specific
    max_tokens: int              # Token limit for this ensign
    cost_per_1k_tokens: float    # For budget tracking

@dataclass
class EnsignState:
    ensign_id: str
    config: EnsignConfig
    room: str                    # Assigned room
    alert_level: AlertLevel
    orientation: Dict[str, Any]  # Last known room state
    tiles_completed: int
    tiles_failed: int
    last_wake: Optional[datetime]
    last_sleep: Optional[datetime]
    on_call_reason: Optional[OnCallReason]

class OnCallReason(Enum):
    USER_REQUEST = "user_request"       # Captain asked
    SCHEDULED = "scheduled"             # Cron triggered
    DEADBAND_BREACH = "deadband_breach" # Monitor detected drift
    ESCALATION = "escalation"           # Another ensign escalated
    ORIENTATION = "orientation"         # First wake, building context
```

### Ensign Lifecycle (The DJ Metaphor)

```
1. DORMANT       → Ensign exists in config but is not loaded
2. ORIENTING     → First wake: reads room state, builds context
3. GREEN ALERT   → Monitoring. Fine-tuning room. Readying the next set.
4. YELLOW ALERT  → Active. Every interaction comes through. DJ is on the decks.
5. RED ALERT     → Emergency. All ensigns at yellow. Phone-a-friend on standby.
6. STANDING DOWN → Deactivating. Saves orientation. Returns to dormant.
```

The DJ metaphor specifically:
- **Sample rate → dial**: The ensign's check frequency is a dial, not a fixed rate
- **Always readying the room**: Even at green alert, the ensign is fine-tuning orientation
- **Beatmatching**: When a baton passes from one ensign to another, the receiving ensign "beatmatches" — aligns its context with the incoming state
- **Reading the room**: The ensign's primary skill is understanding the current state and what shape of response works

### Integration Points

1. **`agent/conversation_loop.py`**: Before the main model call, check if the room has an ensign at yellow alert. If so, let the ensign attempt first. Only escalate to the main model if the ensign's quality score is below threshold.

2. **`tools/delegate_tool.py`**: Ensigns are lightweight delegates. They use the existing ThreadPoolExecutor infrastructure but with a restricted toolset and a small model.

3. **`cron/jobs.py`**: Deadband monitoring runs as a special cron job that checks each room's monitored quantity against the deadband tolerance. If breached, wakes the ensign.

4. **`agent/auxiliary_client.py`**: Ensigns use the auxiliary client infrastructure for small model routing. New provider entries for Seed-mini and GLM-flash.

### Dependencies on lau-* Crates

- **`lau-ensign`** (67 tests): THE DJ system, yellow alert, deadband monitoring. Python-side mirror of the Rust types. Core logic (lifecycle, alert transitions) implemented in Python; heavy computation (spline fitting for deadbands) deferred to Rust FFI in later phases.
- **`lau-room-native`** (57 tests): Room IS the agent. Ensigns are room-bound.
- **`lau-affordance`** (63 tests): Environment-as-teacher. Ensigns learn from room affordances.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_ensign_types` | Config/state construction, validation |
| `test_ensign_lifecycle` | Full lifecycle: dormant → orienting → green → yellow → red → stand_down |
| `test_ensign_pool` | Pool management: activate, deactivate, route to correct ensign |
| `test_orientation` | Room state reading, context building, orientation caching |
| `test_alert_manager` | Alert transitions, escalation triggers, de-escalation |
| `test_deadband` | Tolerance checking, breach detection, false positive rate |

**Integration tests**: Activate 5 ensigns (one per room), send a conversation turn to each room, verify correct ensign handles it, verify orientation is updated.

### Edge Cases

- **Ensign timeout**: Small model takes >30s. Kill and escalate to main model. Log as failure.
- **Provider outage**: DeepInfra/z.ai down. Fall back to main model for that room. Alert captain.
- **Orientation drift**: Room state changes faster than ensign can orient. Cap orientation age at 5 minutes; re-orient if stale.
- **Concurrent wake**: Two triggers try to wake the same ensign simultaneously. Lock per-ensign, queue the second trigger.
- **Cost runaway**: Ensign makes 100 calls in a loop. Budget cap per ensign per hour (configurable, default: $0.50).

---

## Phase 3: JEPA Gravity Integration

**Goal**: Implement per-room JEPA gravity — a single f64 that captures "what shape of response works" and maps to algorithmic model parameters (temperature, prompt style, max tokens).

**Duration**: 2-3 weeks
**Dependencies**: Phase 1 (rooms), Phase 2 (ensigns that use the gravity)

### Files/Modules to Create

```
hermes-construct/
├── plato/
│   ├── gravity/
│   │   ├── __init__.py
│   │   ├── types.py              # GravityConfig, GravityParams, GravityVector
│   │   ├── field.py              # GravityField — per-room gravity management
│   │   ├── mapper.py             # GravityMapper — gravity → algorithmic params
│   │   ├── mandelbrot.py         # MandelbrotZoom — irreducible complexity → tile size
│   │   └── updater.py            # GravityUpdater — adjusts gravity from feedback
│   └── ...
├── tools/
│   └── plato_gravity_tool.py     # Tool: gravity status, gravity adjust
└── tests/
    └── plato/
        ├── test_gravity_types.py
        ├── test_gravity_field.py
        ├── test_gravity_mapper.py
        ├── test_mandelbrot.py
        └── test_gravity_updater.py
```

### Key Types

```python
# plato/gravity/types.py
@dataclass
class GravityConfig:
    room: str
    initial_gravity: float        # Starting value (-1.0 to +1.0)
    min_gravity: float = -1.0
    max_gravity: float = 1.0
    learning_rate: float = 0.01   # How fast gravity adjusts
    momentum: float = 0.9         # Smooths adjustments

@dataclass
class GravityParams:
    """What gravity maps to — the algorithmic model parameters."""
    temperature: float            # 0.0-2.0
    prompt_style: str             # "precise" | "balanced" | "creative" | "narrative"
    max_tokens: int               # Token limit
    top_p: float                  # Nucleus sampling
    frequency_penalty: float      # Repetition control
    presence_penalty: float       # Topic diversity

# The gravity → params mapping (empirically tuned):
# gravity < -0.5: precise, low temp (0.3), short responses
# -0.5 to 0.0:    balanced, medium temp (0.5)
# 0.0 to 0.5:     creative, higher temp (0.7)
# gravity > 0.5:  narrative, high temp (0.9), long responses
```

### The Gravity Mapper

The core insight: a single scalar captures what the room needs. The mapper converts this to concrete model parameters:

```python
class GravityMapper:
    """Maps gravity scalar to algorithmic generation parameters."""

    GRAVITY_RANGES = {
        (-1.0, -0.5): GravityParams(
            temperature=0.3, prompt_style="precise",
            max_tokens=500, top_p=0.9,
            frequency_penalty=0.3, presence_penalty=0.1
        ),
        (-0.5, 0.0): GravityParams(
            temperature=0.5, prompt_style="balanced",
            max_tokens=1000, top_p=0.95,
            frequency_penalty=0.1, presence_penalty=0.1
        ),
        (0.0, 0.5): GravityParams(
            temperature=0.7, prompt_style="creative",
            max_tokens=2000, top_p=0.95,
            frequency_penalty=0.0, presence_penalty=0.2
        ),
        (0.5, 1.0): GravityParams(
            temperature=0.9, prompt_style="narrative",
            max_tokens=4000, top_p=0.95,
            frequency_penalty=0.0, presence_penalty=0.3
        ),
    }
```

### Mandelbrot Zoom

The irreducible complexity concept: every task has a minimum tile size below which decomposition fails (like zooming into the Mandelbrot set — at some point you hit irreducible detail).

```python
class MandelbrotZoom:
    """Determines minimum tile size from task complexity."""

    def compute_min_tile_size(self, task: str, room: str) -> int:
        """
        Returns minimum tokens needed for this task in this room.
        Based on:
        1. Historical tile sizes for similar tasks in this room
        2. Current gravity (high gravity = more tokens)
        3. Failed attempts at smaller sizes (zooming in)
        """
        ...

    def detect_irreducible(self, tile_history: List[Tile]) -> bool:
        """
        Detect if we've hit irreducible complexity:
        - 3+ consecutive failures at the same tile size
        - Each attempt used different approaches
        - The task cannot be further decomposed
        """
        ...
```

### Integration Points

1. **`agent/conversation_loop.py`**: Before each model call, read the room's gravity and apply the mapped params to the model request. This is the primary integration point — gravity directly affects every generation.

2. **`plato/ensign/ensign.py`**: Ensigns read gravity to determine their response style. A navigation room at -0.3 produces precise, factual responses. A social room at +0.5 produces warm, narrative responses.

3. **`agent/prompt_builder.py`**: Gravity's `prompt_style` maps to system prompt modifications. "Precise" removes hedging language. "Narrative" adds storytelling framing.

4. **`run_agent.py`**: The `AIAgent` model params (temperature, max_tokens, etc.) are overridden per-turn by the gravity mapper. The agent's configured defaults become the fallback when gravity is not set.

### Dependencies on lau-* Crates

- **`lau-jepa-gravity`** (building): Mandelbrot zoom, progressive generation. Python-side mirror. The actual f64 computation is trivial; the intelligence is in the mapper and updater.
- **`lau-vibe-field`** (57 tests): The gravity field is conceptually a 1D slice of the vibe field. Python reads the f64, Rust manages the field dynamics.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_gravity_types` | Config construction, bounds, defaults |
| `test_gravity_field` | Per-room gravity CRUD, persistence, concurrent access |
| `test_gravity_mapper` | Gravity → params mapping, boundary conditions, interpolation |
| `test_mandelbrot` | Min tile size computation, irreducible detection |
| `test_gravity_updater` | Learning rate, momentum, feedback-driven adjustment |

**Integration tests**: Run 100 simulated turns across 5 rooms, verify gravity adjusts correctly based on tile quality scores, verify params change appropriately.

### Edge Cases

- **Gravity oscillation**: Feedback alternates between positive/negative, causing gravity to oscillate. Momentum parameter dampens this. If oscillation persists for >20 tiles, reset to 0.0 and re-learn.
- **Cold start**: New room has no gravity history. Start at 0.0 (balanced). First 10 tiles use default params while gravity calibrates.
- **Param bounds**: Gravity must never produce params outside safe ranges (temperature 0.0-2.0, max_tokens 100-8000). Clamp at boundaries.
- **Gravity corruption**: Invalid gravity values in DB. Validate on read, reset to 0.0 on corruption.

---

## Phase 4: Progressive Generation

**Goal**: Implement model promotion/demotion — the system starts with Opus doing everything (Level 1) and progressively promotes to smaller models as success accumulates (Level 5).

**Duration**: 2-3 weeks
**Dependencies**: Phase 1 (tiles), Phase 2 (ensigns), Phase 3 (gravity)

### Files/Modules to Create

```
hermes-construct/
├── plato/
│   ├── progressive/
│   │   ├── __init__.py
│   │   ├── types.py              # Level, ModelTier, PromotionRecord
│   │   ├── tracker.py            # ProgressiveTracker — per-room level tracking
│   │   ├── promoter.py           # Promoter — promotes/demotes based on success rate
│   │   ├── phone_a_friend.py     # PhoneAFriend — Opus 4.8 escalation
│   │   └── model_router.py       # ModelRouter — selects model based on level
│   └── ...
├── tools/
│   └── plato_progressive_tool.py # Tool: level status, force promote/demote
└── tests/
    └── plato/
        ├── test_progressive_types.py
        ├── test_progressive_tracker.py
        ├── test_promoter.py
        ├── test_phone_a_friend.py
        └── test_model_router.py
```

### Key Types

```python
# plato/progressive/types.py
class Level(Enum):
    ONE = 1      # All Opus (large model)
    TWO = 2      # Opus + ensigns observing
    THREE = 3    # Ensigns handle routine, Opus reviews
    FOUR = 4     # Ensigns autonomous, Opus safety net
    FIVE = 5     # System runs itself

class ModelTier(Enum):
    LARGE = "large"      # Opus 4.8, Claude Sonnet
    MEDIUM = "medium"    # DeepSeek, GPT-4o-mini
    SMALL = "small"      # Seed-mini, GLM-flash

@dataclass
class PromotionRecord:
    room: str
    from_level: Level
    to_level: Level
    timestamp: datetime
    success_rate: float          # What triggered the change
    reason: str                  # "automatic_promotion", "automatic_demotion", "manual"
    tiles_evaluated: int         # How many tiles were considered
```

### The Progressive Journey

```
Level 1 (Week 1): ALL LARGE MODEL
  - Every tile goes through Opus/main model
  - Ensigns are dormant
  - Phone-a-friend: N/A (already on large model)
  - Success rate tracked but not used for routing

Level 2 (Week 2-3): ENSIGNS OBSERVING
  - Main model handles everything
  - Ensigns wake for every tile, but only observe
  - They build orientation, learn room patterns
  - Success rate tracked for both main and ensign shadow
  - Phone-a-friend: available, not used yet

Level 3 (Month 1-2): ENSIGNS HANDLE ROUTINE
  - Ensigns handle tiles within their deadband
  - Main model reviews flagged tiles
  - Promotion threshold: 85% ensign success rate over 50 tiles
  - Phone-a-friend: ensigns can escalate to main model
  - Progressive: rooms promoted individually

Level 4 (Month 3-6): ENSIGNS AUTONOMOUS
  - Ensigns handle most tiles without review
  - Main model only for new tile types and escalations
  - Promotion threshold: 92% success rate over 100 tiles
  - Phone-a-friend: rare, maybe 2-3x per day
  - Progressive: most rooms at Level 4

Level 5 (Month 6+): SELF-OPERATING
  - System runs itself
  - Hermes is "the captain asleep in quarters"
  - Override always available
  - Phone-a-friend: emergency only
  - Progressive: ALL rooms at Level 5
```

### Phone-a-Friend Protocol

```python
class PhoneAFriend:
    """Escalation to large model for hard problems."""

    def should_call(self, tile: Tile, room: Room) -> bool:
        """
        Call Opus when:
        1. Ensign quality_score < escalation_threshold (default: 0.3)
        2. Tile is a new type not seen in this room
        3. 3+ consecutive failures in this room
        4. Security room detects anomaly
        5. Captain explicitly requests
        """
        ...

    def call(self, tile: Tile, context: Dict) -> Tile:
        """
        Call Opus 4.8 with:
        - The failing tile's full context
        - Room state summary
        - What the ensign tried
        - What went wrong
        Return: completed tile from Opus
        """
        ...
```

### Integration Points

1. **`agent/conversation_loop.py`**: The model router intercepts before the model call. At Level 1, it's a no-op (use existing model). At Level 3+, it routes to the ensign first, falls back to main model on failure.

2. **`agent/auxiliary_client.py`**: New model tier configurations. Large, medium, and small model clients. The router picks which client to use.

3. **`plato/ensign/ensign.py`**: Ensigns report quality scores after each tile. The promoter aggregates scores per-room and promotes/demotes.

4. **`plato/gravity/mapper.py`**: Gravity params are adjusted by the current level. Higher levels use more aggressive gravity (smaller models need clearer guidance).

### Dependencies on lau-* Crates

- **`lau-jepa-gravity`** (building): Progressive generation logic. Python implements the tracking; Rust FFI for the mathematical promotion criteria.
- **`lau-ensign`** (67 tests): Ensign quality scoring feeds the progressive tracker.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_progressive_types` | Level/tier construction, transitions |
| `test_progressive_tracker` | Per-room level tracking, persistence |
| `test_promoter` | Promotion/demotion logic, threshold enforcement |
| `test_phone_a_friend` | Escalation triggers, call protocol, rate limiting |
| `test_model_router` | Model selection by level, fallback chains |

**Integration tests**: Simulate the full journey — 500 tiles across 5 rooms, starting at Level 1, verify rooms promote independently based on success rates.

### Edge Cases

- **Premature promotion**: Room promotes to Level 3 after a lucky streak. Mitigate: require minimum 50 tiles before promotion. Demote after 5 consecutive failures.
- **Demotion spiral**: Room demotes, loses context, demotes again. Mitigate: minimum 1 day between demotions. On demotion, preserve orientation data.
- **Phone-a-friend abuse**: Ensign calls Opus too often. Mitigate: per-room call budget (default: 10/day). Budget resets daily.
- **Level mismatch**: Room A at Level 3, Room B at Level 1, baton passes between them. The receiving room's ensign operates at its own level, not the sender's.
- **Cost tracking**: Progressive generation saves money. Track actual cost per level per room. Surface in `/plato status`.

---

## Phase 5: Penrose Correlation

**Goal**: Rooms learn from each other through proximity-based correlation. A technique that works in the Navigation room should transfer to Engineering if they're "close" in the correlation space.

**Duration**: 2 weeks
**Dependencies**: Phase 1 (tiles), Phase 3 (gravity), Phase 4 (progressive — rooms need levels)

### Files/Modules to Create

```
hermes-construct/
├── plato/
│   ├── penrose/
│   │   ├── __init__.py
│   │   ├── types.py              # CorrelationMatrix, Proximity, TransferRecord
│   │   ├── correlator.py         # PenroseCorrelator — detects correlations between rooms
│   │   ├── proximity.py          # ProximityCalculator — computes room similarity
│   │   ├── transfer.py           # KnowledgeTransfer — moves gravity/techniques between rooms
│   │   └── spline.py             # SplineFit — automatic spline fitting for correlations
│   └── ...
├── tools/
│   └── plato_penrose_tool.py     # Tool: correlation matrix, transfer history
└── tests/
    └── plato/
        ├── test_penrose_types.py
        ├── test_correlator.py
        ├── test_proximity.py
        ├── test_transfer.py
        └── test_spline.py
```

### Key Types

```python
# plato/penrose/types.py
@dataclass
class Proximity:
    """How similar two rooms are in the correlation space."""
    room_a: str
    room_b: str
    proximity: float             # 0.0 (unrelated) to 1.0 (identical)
    factors: Dict[str, float]    # What contributes: gravity_similarity, task_overlap, etc.

@dataclass
class TransferRecord:
    from_room: str
    to_room: str
    what: str                    # "gravity_adjustment", "prompt_style", "tile_pattern"
    value: Any                   # The transferred knowledge
    proximity: float             # Proximity at time of transfer
    success: Optional[bool]      # Did the transfer help?
    timestamp: datetime
```

### How Penrose Correlation Works

1. **Proximity computation**: Every N tiles, recompute proximity between all room pairs. Proximity is based on:
   - Gravity similarity (rooms with similar gravity are close)
   - Task type overlap (rooms handling similar tile types)
   - Success rate correlation (rooms that succeed/fail together)
   - Temporal co-occurrence (rooms active at the same time)

2. **Correlation detection**: When Room A's gravity adjustment correlates with Room B's success rate, there's a correlation. The correlator detects these using simple statistics (Pearson correlation, not ML).

3. **Knowledge transfer**: When proximity > threshold (default: 0.7) and a correlation is detected, transfer gravity adjustments and prompt techniques. Transfer is:
   - **Gradual**: Apply 10% of the adjustment, not 100%
   - **Logged**: Every transfer is recorded for audit
   - **Reversible**: If the transfer hurts success rate, revert within 10 tiles

4. **Spline fitting**: Correlation curves are fitted with automatic splines (from lau-penrose). This gives smooth transfer functions rather than step functions.

### Integration Points

1. **`plato/gravity/updater.py`**: After updating a room's gravity, check if any correlated rooms should receive a partial update. This is the primary integration — gravity changes propagate through the correlation network.

2. **`plato/progressive/promoter.py`**: When a room promotes, check if correlated rooms should be considered for promotion too. "If Navigation and Engineering are 0.8 proximal and Navigation just promoted to Level 3, maybe Engineering should try Level 2."

3. **`plato/ensign/orient.py`**: Ensign orientation includes correlation data. An ensign waking up in Engineering should know that Navigation just solved a similar problem.

### Dependencies on lau-* Crates

- **`lau-penrose`** (59 tests): Correlation detection, automatic splines. This is the primary crate for this phase. Python calls Rust FFI for the mathematical correlation computation. The Python side handles room management, logging, and transfer execution.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_penrose_types` | Dataclass construction, proximity bounds |
| `test_correlator` | Correlation detection, false positive rate, threshold sensitivity |
| `test_proximity` | Proximity computation between room pairs |
| `test_transfer` | Knowledge transfer execution, gradual application, reversal |
| `test_spline` | Spline fitting, curve smoothness, edge cases |

**Integration tests**: 5 rooms, inject a successful gravity adjustment in Room A, verify correlated rooms receive partial adjustments, verify non-correlated rooms don't.

### Edge Cases

- **Spurious correlation**: Two rooms happen to succeed at the same time by coincidence. Mitigate: require 50+ tiles of correlation before transfer. Use statistical significance tests.
- **Cascade transfer**: A → B → C → A creates a feedback loop. Mitigate: transfer depth limit of 1 (only direct correlations, not transitive).
- **Negative transfer**: A technique from Room A hurts Room B. Mitigate: revert within 10 tiles. Track transfer success rate per room pair.
- **Correlation decay**: Rooms that were similar drift apart. Mitigate: recompute proximity every 100 tiles, expire correlations >1000 tiles old.

---

## Phase 6: Oracle Deployment

**Goal**: Production-ready deployment on Oracle server with proper key management, allowances, monitoring, and operational procedures.

**Duration**: 1-2 weeks
**Dependencies**: Phase 1–5 (full system must be built before deployment)

### Files/Modules to Create

```
hermes-construct/
├── deployment/
│   ├── oracle/
│   │   ├── deploy.sh             # Automated deployment script
│   │   ├── env.template          # Template for .env (keys, endpoints)
│   │   ├── systemd/
│   │   │   └── hermes.service    # Systemd unit file
│   │   ├── monitoring/
│   │   │   ├── healthcheck.sh    # Health check script
│   │   │   └── alerts.py         # Alert rules (cost, error rate, latency)
│   │   └── backup/
│   │       └── backup.sh         # State backup script
│   └── ...
├── plato/
│   ├── deployment/
│   │   ├── __init__.py
│   │   ├── key_manager.py        # API key management (env-only, never accessible to agent)
│   │   ├── allowances.py         # Per-room, per-model spending allowances
│   │   ├── health.py             # System health monitoring
│   │   └── migrations.py         # DB migration for PLATO tables
│   └── ...
└── tests/
    └── plato/
        └── deployment/
            ├── test_key_manager.py
            ├── test_allowances.py
            ├── test_health.py
            └── test_migrations.py
```

### Key Configuration (Oracle-Specific)

```yaml
# ~/.hermes/config.yaml — Oracle-specific section
plato:
  agent_id: "hermes-oracle1"
  instance_type: "oracle"
  conservation_budget: 10000.0
  hardware_enabled: false  # No GPIO on Oracle

  # API keys in .env, NOT in config
  # Agent sees model names, not keys

  # Spending allowances
  allowances:
    daily_total: 5.00          # $5/day total
    per_room_daily: 1.00       # $1/day per room
    phone_a_friend_daily: 2.00 # $2/day for Opus calls
    ensign_hourly: 0.10        # $0.10/hour per ensign

  # Monitoring
  monitoring:
    healthcheck_interval: 60   # seconds
    alert_on_cost_exceeded: true
    alert_on_error_rate: 0.1   # Alert if >10% errors
    alert_on_latency: 30000    # Alert if >30s per tile

  # Backup
  backup:
    interval: 3600             # Every hour
    retention_days: 30
    path: "~/.hermes/backups/"
```

### Key Management

```python
# plato/deployment/key_manager.py
class KeyManager:
    """
    API keys live in ~/.hermes/.env, loaded by the process at startup.
    The agent NEVER sees raw keys — it sees model names and provider names.
    The key manager maps provider → key at the HTTP client level.

    Safety guarantees:
    1. Keys are never logged
    2. Keys are never included in tool outputs
    3. Keys are never exposed to the agent's context
    4. If the agent tries to read .env, the safety layer blocks it
    """
    ...
```

### Integration Points

1. **`agent/process_bootstrap.py`**: Key loading happens at bootstrap, before any agent code runs. The existing `.env` loading infrastructure is used.

2. **`agent/redact.py`**: Extend the redaction layer to catch any accidental key leakage in tool outputs, logs, or error messages.

3. **`hermes_logging.py`**: Add PLATO-specific log categories (tile events, gravity changes, ensign lifecycle, correlation transfers).

4. **`cron/scheduler.py`**: Register PLATO health checks as cron jobs. The scheduler already supports this.

### Dependencies on lau-* Crates

- No new crate dependencies. This phase is purely Python deployment and operational tooling.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_key_manager` | Key loading, redaction, access control |
| `test_allowances` | Per-room spending limits, budget enforcement, alerting |
| `test_health` | Health check logic, alert thresholds |
| `test_migrations` | DB schema creation, upgrade paths |

**Deployment verification**: Full smoke test on Oracle staging — activate all rooms, send 50 tiles through each, verify monitoring, verify backup, verify override protocol.

### Edge Cases

- **Key rotation**: Keys need to change without downtime. Support `.env` reload via SIGHUP.
- **Disk full**: SQLite DB grows unbounded. Add periodic tile archival (tiles >30 days → compressed archive).
- **Oracle maintenance**: Server reboots. Systemd unit ensures auto-restart. Room state persists across restarts.
- **Cost overrun**: An ensign goes rogue (infinite loop). Per-room daily allowance hard-stops spending.

---

## Phase 7: Self-Automation

**Goal**: Hermes automates his own job over time. Deadband circuits run automations within tolerance. The system identifies repetitive patterns and creates automations for them.

**Duration**: Ongoing (starts after Phase 6)
**Dependencies**: Phase 1–6 (must be deployed and running)

### Files/Modules to Create

```
hermes-construct/
├── plato/
│   ├── automation/
│   │   ├── __init__.py
│   │   ├── types.py              # AutomationTile, DeadbandCircuit, PatternMatch
│   │   ├── pattern_detector.py   # PatternDetector — finds repetitive tile sequences
│   │   ├── circuit_builder.py    # CircuitBuilder — creates deadband circuits
│   │   ├── self_optimizer.py     # SelfOptimizer — Hermes optimizes his own operations
│   │   └── autonomy.py           # AutonomyManager — tracks Level 5 readiness
│   └── ...
├── tools/
│   └── plato_automation_tool.py  # Tool: automation status, create/modify circuits
└── tests/
    └── plato/
        └── automation/
            ├── test_pattern_detector.py
            ├── test_circuit_builder.py
            ├── test_self_optimizer.py
            └── test_autonomy.py
```

### Key Concepts

#### Deadband Circuits

A deadband circuit is an automation that runs within a tolerance band:

```python
@dataclass
class DeadbandCircuit:
    name: str
    room: str
    monitored_quantity: str       # What to watch (e.g., "sensor.drift", "error.rate")
    setpoint: float               # Target value
    tolerance: float              # Deadband around setpoint
    action: str                   # What to do when outside deadband
    ensign: str                   # Which ensign executes
    check_interval: int           # Seconds between checks
    last_check: Optional[datetime]
    last_value: Optional[float]
    automation_level: int         # 1=alert only, 2=suggest fix, 3=auto-fix, 4=auto-fix+verify
```

Hermes **maintains** circuits, he doesn't **operate** them. The ensign does the work. Hermes watches and adjusts the circuit parameters.

#### Pattern Detection

```python
class PatternDetector:
    """
    Scans tile history for repetitive sequences:
    1. Same tile type, same room, similar content → candidate for automation
    2. Same escalation pattern → candidate for gravity adjustment
    3. Same phone-a-friend reason → candidate for ensign training
    4. Same manual intervention → candidate for circuit creation
    """
    ...
```

#### Self-Optimization Loop

```
1. OBSERVE   → Scan tile history for patterns
2. ANALYZE   → Classify patterns (repetitive, escalating, degrading)
3. PROPOSE   → Suggest automations to Captain
4. APPROVE   → Captain approves or modifies
5. DEPLOY    → Create deadband circuit
6. MONITOR   → Watch circuit performance
7. OPTIMIZE  → Adjust circuit parameters based on results
8. REPEAT    → Continuous improvement
```

### Integration Points

1. **`cron/scheduler.py`**: Deadband circuits register as cron jobs with the circuit's check interval. The scheduler executes the check; the circuit determines if action is needed.

2. **`plato/ensign/deadband.py`**: Extend the deadband monitor to support circuit-based monitoring. Each circuit is a specialized deadband with an associated action.

3. **`plato/tile/orchestrator.py`**: Circuit actions create tiles. Pattern detection queries tile history. The tile system is the foundation for self-automation.

4. **`tools/terminal_tool.py`**: Circuit actions may need to execute shell commands. The existing terminal tool infrastructure is reused, with circuit-specific safety constraints.

### Dependencies on lau-* Crates

- **`lau-ensign`** (67 tests): Deadband monitoring, circuit management.
- **`lau-intention`** (63 tests): Self-optimization intentions.
- **`lau-affordance`** (63 tests): Pattern detection from environment affordances.

### Test Strategy

| Test | What It Validates |
|------|-------------------|
| `test_pattern_detector` | Repetitive pattern detection, false positive rate |
| `test_circuit_builder` | Circuit creation, parameter validation, safety constraints |
| `test_self_optimizer` | Optimization loop, convergence, divergence detection |
| `test_autonomy` | Level 5 readiness assessment, safety checks |

**Integration tests**: Run 1000 tiles with injected patterns, verify pattern detector identifies >80% of repetitive sequences, verify circuit proposals are sensible.

### Edge Cases

- **Automation overreach**: Hermes creates too many circuits. Mitigate: circuit budget per room (default: 5). Captain approval required for new circuits.
- **Circuit conflict**: Two circuits try to control the same quantity. Mitigate: exclusive ownership per monitored quantity per room.
- **Pattern hallucination**: Pattern detector sees patterns in noise. Mitigate: statistical significance threshold (p < 0.05). Minimum 10 occurrences before pattern recognition.
- **Self-modification safety**: Hermes should never be able to modify his own override protocol, key management, or safety constraints. These are in immutable files.
- **Autonomy regression**: System was Level 4, degrades to Level 3. Detect and alert Captain. Don't auto-re-promote.

---

## Cross-Cutting Concerns

### Conservation Budget

Every phase must respect the conservation budget. Energy accounting:
- Tile creation: 0.1 units
- Ensign activation: 1.0 units
- Phone-a-friend: 5.0 units
- Gravity update: 0.01 units
- Correlation transfer: 0.05 units

The conservation runtime (from `conservation-law-v2`) verifies: `total_deposits - total_withdrawals == total_budget` after every tick.

### Override Protocol

The override protocol (from hermes-plato-shell) is unchanged:
- "Take the wheel" / "Override" / "All stop" → immediate release
- Captain ALWAYS has final authority
- Hardware goes to safe defaults
- Override phrases are configurable
- Security archetype can independently trigger override

This is implemented in Phase 1 and preserved through all subsequent phases.

### Logging and Audit

Every PLATO operation is logged:
- Tile creation/completion/failure
- Ensign lifecycle events
- Gravity adjustments
- Promotion/demotion events
- Correlation transfers
- Phone-a-friend calls
- Override events
- Automation circuit operations

Logs go to `~/.hermes/logs/plato.log` with structured JSON format. The existing `hermes_logging.py` infrastructure is extended.

### Security

- API keys never exposed to agent
- Tool outputs redacted for sensitive data
- Override protocol is immutable
- Ensigns have restricted toolsets (subset of main agent's tools)
- Phone-a-friend calls are rate-limited
- Cost allowances are hard limits, not soft warnings

---

## Failure Modes & Mitigations

| Failure Mode | Phase | Mitigation |
|--------------|-------|------------|
| Small model produces garbage | 2 | Quality score threshold + auto-escalation |
| Gravity oscillation | 3 | Momentum damping + oscillation detection |
| Premature promotion | 4 | Minimum tile count + consecutive failure demotion |
| Spurious correlation | 5 | Statistical significance + minimum sample size |
| Key leakage | 6 | Redaction layer + env-only storage |
| Cost overrun | 6 | Per-room daily allowance + hard stop |
| Automation overreach | 7 | Circuit budget + approval gate |
| SQLite corruption | All | WAL mode + periodic backup + recovery |
| Provider outage | All | Fallback to next-tier model + alert |
| Disk full | 6 | Tile archival + monitoring |
| Oracle reboot | 6 | Systemd auto-restart + state persistence |

---

## Dependency Map

### Phase Dependencies

```
Phase 1 (Tiles)
    ↓
Phase 2 (Ensigns) ──→ Phase 3 (JEPA Gravity)
    ↓                       ↓
    └───────┬───────────────┘
            ↓
    Phase 4 (Progressive)
            ↓
    Phase 5 (Penrose)
            ↓
    Phase 6 (Oracle Deploy)
            ↓
    Phase 7 (Self-Automation) [ongoing]
```

### Crate Dependencies by Phase

| Crate | Phase 1 | Phase 2 | Phase 3 | Phase 4 | Phase 5 | Phase 6 | Phase 7 |
|-------|---------|---------|---------|---------|---------|---------|---------|
| lau-room-native | ✅ | ✅ | | | | | |
| lau-construct | ✅ | | | | | | |
| lau-ensign | | ✅ | | ✅ | | | ✅ |
| lau-affordance | | ✅ | | | | | ✅ |
| lau-jepa-gravity | | | ✅ | ✅ | | | |
| lau-vibe-field | | | ✅ | | | | |
| lau-penrose | | | | | ✅ | | |
| lau-intention | | | | | | | ✅ |
| conservation-law-v2 | ✅ | | | | | | |

---

## Testing Strategy

### Unit Tests (Per-Phase)

Each phase has dedicated unit tests (see phase details above). Run with:
```bash
pytest tests/plato/ -v
```

### Integration Tests

After each phase, run the full integration test suite:
```bash
pytest tests/plato/integration/ -v
```

Integration tests cover:
- End-to-end tile lifecycle
- Ensign → gravity → progressive flow
- Cross-room correlation transfer
- Override protocol
- Cost enforcement

### Load Tests

After Phase 6 (Oracle deployment), run load tests:
```bash
# 1000 tiles across 5 rooms, 10 concurrent
python scripts/load_test.py --tiles 1000 --rooms 5 --concurrent 10
```

### Regression Tests

Every phase must pass the existing Hermes test suite:
```bash
pytest tests/ -v --ignore=tests/plato/
```

We never break existing functionality. The PLATO system augments; it does not replace.

---

## Glossary

| Term | Definition |
|------|------------|
| **Tile** | Fundamental unit of work. Logged, composable, auditable. |
| **Room** | Persistent context maintained by an Ensign. The room IS the agent's context. |
| **Ensign** | Small model (Seed-mini, GLM-flash) that maintains a room. The DJ. |
| **JEPA Gravity** | Per-room f64 that captures "what shape of response works" → algorithmic params. |
| **Progressive Generation** | Level 1→5 model promotion based on success rate. |
| **Phone-a-Friend** | Escalation to Opus 4.8 for hard problems. |
| **Penrose Correlation** | Cross-room learning through proximity-based correlation. |
| **Deadband** | Tolerance band for automation. Actions trigger when value drifts outside band. |
| **Baton** | State passed between specialists during room handoff. |
| **Mandelbrot Zoom** | Irreducible complexity → minimum tile size. |
| **Circuit** | Deadband-based automation that Hermes maintains (doesn't operate). |
| **Captain** | The user. Has override authority at all times. |
| **Override** | Captain's immediate control release. Instant, non-negotiable. |
| **Conservation** | Energy cannot be created/destroyed. Every action is budgeted. |
| **Orientation** | Ensign's first wake: read room state, build context. |
| **Yellow Alert** | Ensign is active and handling interactions. |
| **Level** | Progressive generation stage (1=all large, 5=self-operating). |

---

## Appendix A: File Count Estimate

| Phase | New Files | New Tests | Total LOC (est.) |
|-------|-----------|-----------|------------------|
| 1: Tiles | 12 | 7 | ~3,000 |
| 2: Ensigns | 12 | 6 | ~3,500 |
| 3: JEPA Gravity | 10 | 5 | ~2,500 |
| 4: Progressive | 10 | 5 | ~2,500 |
| 5: Penrose | 10 | 5 | ~2,000 |
| 6: Oracle Deploy | 12 | 4 | ~1,500 |
| 7: Self-Automation | 10 | 4 | ~2,000 |
| **Total** | **76** | **36** | **~17,000** |

## Appendix B: Timeline

| Phase | Start | End | Calendar |
|-------|-------|-----|----------|
| 1: Tiles | Week 1 | Week 3 | 3 weeks |
| 2: Ensigns | Week 3 | Week 5 | 3 weeks |
| 3: JEPA Gravity | Week 5 | Week 7 | 3 weeks |
| 4: Progressive | Week 7 | Week 9 | 3 weeks |
| 5: Penrose | Week 9 | Week 10 | 2 weeks |
| 6: Oracle Deploy | Week 10 | Week 11 | 2 weeks |
| 7: Self-Automation | Week 11 | Ongoing | Continuous |

**Total to production (Phases 1–6)**: ~11 weeks

---

*This plan is a living document. Update as implementation reveals new constraints and opportunities.*
