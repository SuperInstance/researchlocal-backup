# hermes-construct — Product Roadmap

**Author**: SuperInstance  
**Date**: 2026-06-01  
**Status**: Living document — update as implementation reveals constraints

---

## The Vision

hermes-construct becomes **the plug-and-play AI agent** that works on the day you install it, grows with every task, and never asks you to choose between capability and simplicity.

The agent boots with a first-run wizard. The human picks their role. The agent loads the modules that role needs and nothing else. From that moment, the agent watches what it does and what it doesn't do, and quietly loads new capabilities as tasks demand them and unloads them when they're no longer needed. The heavy lifting — PLATO spectral monitoring, crackle detection, topological probing, mathematical packs — lives in modules that the agent selects autonomously. A single binary. A single database. The complexity is hidden but accessible.

This is not a chatbot with plugins. This is a room-native agent that operates tiles, reads its own channel, and maintains a conservation budget across everything it does.

---

## Architecture Principles

Before the phases, the principles that govern every architectural decision:

1. **Every operation is a tile.** Nothing happens outside a tile. Tiles are logged, composable, and auditable. A single user message is a tile. A module load is a tile. An onboarding step is a tile.

2. **Modules are single files.** A module is one file (plus an optional manifest). Adding or removing a module cannot break anything else. The module system enforces this at the ABI boundary.

3. **The agent is self-configuring.** The human picks a role at onboarding. After that, the agent reads each task and loads what it needs. Module selection is not a user action — it is an agent decision, auditable and reversible.

4. **Negative space is tested.** For every capability the agent has, there is a specification of what it must NOT do. The `negative-space-testing` module enforces these constraints at runtime. The agent knows its forbidden behaviors as precisely as its permitted ones.

5. **Conservation is real.** Every operation has an energy cost. The conservation budget is not a metaphor — it is enforced per tile, per module load, per escalation. If the budget is exhausted, the agent degrades gracefully rather than overdraws.

6. **Craze lines are honored.** When the agent falls back, the fallback path is recorded as a first-class artifact — a provenance trail, not an error log. The system's imperfections are its autobiography.

7. **The loop closes.** The agent reads its own tile history (self-reading architecture). The background tick detects correlations between rooms (Penrose). These are not monitoring features — they are how the agent gets smarter without being retrained.

---

## Phase 0: Foundation (Week 1–2)

**Goal**: A clean, stable, ARM-ready fork that can accept PLATO additions without fighting the upstream.

### Why This Phase Exists

hermes-construct is 38 commits behind upstream (NousResearch/hermes-agent). The Oracle ARM cross-compilation, Ensign configs (GLM-flash, Seed-Mini), provenance recording, and graceful degradation work were added on top of a diverging base. Phase 0 closes that gap before any module work begins. A module system built on a shaky foundation is a module system you will rewrite.

### Deliverables

#### Week 1: Rebase and Stabilize

```
Tasks:
1. git fetch upstream && git rebase upstream/main
   - Resolve conflicts in: cli.py, run_agent.py, toolsets.py, hermes_state.py
   - The PLATO additions (plato/, rooms/, ensigns/) should rebase cleanly
     (they live in new directories, no upstream conflict)
   - The Ensign configs in ensigns/ are additive — should also rebase cleanly

2. Regression test:
   - pytest tests/ -v --ignore=tests/plato/ (existing Hermes test suite)
   - All 38+ commits of upstream changes tested against our additions

3. Stabilize provenance recording:
   - Current state: provenance is recorded ad-hoc during tool calls
   - Target: every tool call result includes a ProvenanceRecord
     { model_used, provider, latency_ms, degradation_path: Option<str> }
   - ProvenanceRecord is stored alongside the tile (not in the tile — beside it)
   - If a tool call falls back (e.g., primary model fails → secondary model), 
     the fallback path is the degradation_path — the craze line

4. Stabilize graceful degradation:
   - Current state: degradation is catch-all try/except
   - Target: each degradation path has an explicit DegradationMode enum value
     { ModelFallback, ProviderFallback, CachedResponse, OfflineSkeleton }
   - Every degraded response includes the mode in its metadata
```

#### Week 2: ARM Build Pipeline

```
Target: Single static binary for aarch64-unknown-linux-gnu
Build: cargo build --release --target aarch64-unknown-linux-gnu

Files:
  .github/workflows/arm-build.yml    # CI: ARM cross-compile on every push
  scripts/build-arm.sh               # Local cross-compile script
  scripts/verify-arm.sh              # Smoke test on QEMU

Requirements:
  - Binary size: < 50MB stripped
  - Cold start: < 2s on Oracle ARM (4 cores, 24GB RAM)
  - SQLite WAL: ~50MB for 100K tiles
  - Process memory: ~100MB (no model weights — all remote API)

The Rust binary is the kernel:
  src/main.rs          # Entry point
  src/kernel.rs        # ShellKernel — tile store, room manager, module dispatcher
  src/tile.rs          # Tile types + SQLite store
  src/room.rs          # Room definitions (loaded from JSON in rooms/)
  src/ensign.rs        # Ensign lifecycle
  src/gravity.rs       # JEPA gravity per room → model params
  src/module.rs        # MODULE LOADER (stub for Phase 1)
  src/port.rs          # Port adapters (Telegram v1)
  src/deadband.rs      # Deadband monitoring
  src/conservation.rs  # Budget tracking
```

### Technical Decisions

**Why Rust for the binary?** The Oracle box runs 4 ARM cores and 24GB RAM. No model weights load locally — all inference is remote API. The binary's job is routing between rooms, managing tiles, and calling APIs. Rust gives us a ~10K LOC binary, SQLite WAL performance, and zero GC pauses. The tile store needs to handle 100K tiles at ~50MB — trivial for SQLite on ARM.

**Why not stay Python?** Python is the upstream language and the right choice for the agent logic, plugin ecosystem, and Hermes compatibility. The Rust binary is the *kernel* — the layer below the Python agent that manages rooms, tiles, and modules. The kernel and the agent communicate over a local socket. This architecture lets us ship a single binary that embeds the Python runtime for the agent logic while keeping the kernel in Rust.

**Module stub in Phase 0**: `src/module.rs` is created as a stub with the full module ABI (Phase 1 will implement it). This prevents the Phase 1 work from requiring architectural surgery to the kernel.

### Success Criteria

- `pytest tests/ -v` passes with ≥ 95% of the upstream test suite
- `cargo build --release --target aarch64-unknown-linux-gnu` completes cleanly
- ARM binary starts in < 2s and serves a Telegram message end-to-end
- Provenance is recorded for every tool call (craze lines are visible)
- DegradationMode is explicit in every fallback response

---

## Phase 1: Module System (Week 3–6)

**Goal**: A production-quality plugin/module system with a stable ABI, manifest format, runtime loader, and five working modules.

`★ Insight ─────────────────────────────────────`
The module system is the product's core bet. Every architectural decision here echoes for years. The ABI must be stable: once a module is written against it, it must never need to be rewritten. The manifest format must be machine-readable: the self-configuring agent in Phase 2 will parse manifests to understand what each module can do. Get these two things right and the ecosystem builds itself.
`─────────────────────────────────────────────────`

### Module ABI

A module is a single Python file (`.py`) or Rust shared library (`.so`) with a manifest sidecar (`.module.json`). The kernel discovers modules by scanning the `modules/` directory.

**Python Module Interface:**

```python
# Every module implements this interface
# The module file IS the module — no class wrapper needed

MODULE_MANIFEST = {
    "id": "crackle-runtime",           # Unique stable ID (never changes)
    "version": "1.0.0",                # Semver
    "display_name": "Crackle Runtime", # Human name
    "description": "...",              # One-sentence description
    "capabilities": [                  # Machine-readable capability list
        "pattern.emergence.detect",
        "pattern.emergence.report",
    ],
    "tools": [                         # Tools this module exposes to the agent
        "crackle_scan",
        "crackle_report",
    ],
    "triggers": [                      # When the agent should auto-load this module
        {"task_pattern": "detect.*pattern", "confidence": 0.8},
        {"task_pattern": "emergence.*signal", "confidence": 0.7},
    ],
    "conflicts": [],                   # Module IDs this module cannot coexist with
    "depends": [],                     # Module IDs required before this one loads
    "energy_cost": {                   # Conservation budget per operation
        "load": 1.0,
        "per_tool_call": 0.5,
        "unload": 0.3,
    },
    "rooms": ["science", "engineering"], # Rooms this module is relevant to
    "temperature_range": [-1.0, 0.5],    # JEPA gravity range where this module is effective
    "arm_compatible": true,
    "size_kb": 42,                     # Module size — affects load time on ARM
}

def on_load(kernel: KernelClient) -> None:
    """Called when the module is loaded. Register tools, set up state."""
    kernel.register_tools(MODULE_MANIFEST["tools"], globals())

def on_unload(kernel: KernelClient) -> None:
    """Called before the module is unloaded. Clean up state."""
    pass

def on_task_start(task: TaskContext) -> None:
    """Called when a new task begins. Module can read the task description."""
    pass

def on_task_end(task: TaskContext, result: TaskResult) -> None:
    """Called when a task completes. Module can learn from the outcome."""
    pass

# Tool implementations
def crackle_scan(kernel: KernelClient, **kwargs) -> dict:
    """Scan for emergent patterns in the current room's tile history."""
    ...

def crackle_report(kernel: KernelClient, **kwargs) -> dict:
    """Generate a report of detected patterns."""
    ...
```

**Module Manifest Schema (`.module.json`):**

```json
{
  "$schema": "https://superinstance.io/schemas/module-manifest/v1.json",
  "id": "crackle-runtime",
  "version": "1.0.0",
  "display_name": "Crackle Runtime",
  "description": "Detects emergent patterns in tile histories. Useful for spotting unexpected correlations between rooms, recurring failure modes, and signal-in-noise analysis.",
  "capabilities": [
    "pattern.emergence.detect",
    "pattern.emergence.report"
  ],
  "tools": ["crackle_scan", "crackle_report"],
  "triggers": [
    {"task_pattern": "detect.*pattern", "confidence": 0.8},
    {"task_pattern": "emergence.*signal", "confidence": 0.7},
    {"task_pattern": "anomaly.*detection", "confidence": 0.6}
  ],
  "conflicts": [],
  "depends": [],
  "energy_cost": {"load": 1.0, "per_tool_call": 0.5, "unload": 0.3},
  "rooms": ["science", "engineering"],
  "temperature_range": [-1.0, 0.5],
  "arm_compatible": true,
  "size_kb": 42,
  "author": "SuperInstance",
  "license": "MIT",
  "tags": ["analysis", "patterns", "emergence", "monitoring"]
}
```

### Runtime Module Loader

```
modules/
├── __init__.py
├── loader.py          # ModuleLoader — discover, load, unload, query
├── registry.py        # ModuleRegistry — capability → module mapping
├── resolver.py        # DependencyResolver — topological sort on depends/conflicts
├── sandbox.py         # ModuleSandbox — restricted execution environment
├── budget.py          # ModuleBudget — conservation tracking per module
└── types.py           # ModuleState, ModuleEvent, LoadResult

src/
└── module.rs          # Kernel-side: module socket server, tile events to modules
```

**Module Loader API:**

```python
class ModuleLoader:
    def discover(self, modules_dir: Path) -> List[ModuleManifest]:
        """Scan modules/ directory, parse manifests, return available modules."""
    
    def load(self, module_id: str) -> LoadResult:
        """Load a module. Creates a load tile. Enforces energy budget."""
    
    def unload(self, module_id: str) -> UnloadResult:
        """Unload a module. Calls on_unload. Creates an unload tile."""
    
    def query_capabilities(self, capability: str) -> List[str]:
        """Return module IDs that provide a given capability."""
    
    def query_for_task(self, task_description: str) -> List[ModuleMatch]:
        """Return modules ranked by relevance to a task description."""
    
    def loaded(self) -> List[str]:
        """Return IDs of currently loaded modules."""
    
    def available(self) -> List[ModuleManifest]:
        """Return manifests of all discoverable modules."""
```

### Module Dependency Resolution

The `DependencyResolver` runs a topological sort on the `depends` and `conflicts` fields before any load. If a dependency chain cannot be resolved (circular dependency, hard conflict), the load fails with a specific error and creates a failed tile — it does not partially load.

```
Resolution algorithm:
1. Build dependency graph from all available manifests
2. For requested module M:
   a. Check M's depends — load them first (recursive)
   b. Check M's conflicts — fail if any conflicting module is loaded
   c. Topological sort of the full load chain
   d. Execute loads in topological order
3. On any failure: rollback all partial loads, create failed tile
```

### Module Temperature (Hot / Warm / Cold)

Every loaded module has a temperature:

| Temperature | Meaning | Memory State | Unload Trigger |
|---|---|---|---|
| **Hot** | Actively used this task | Fully loaded in memory | Task completion |
| **Warm** | Used recently, may be needed again | Loaded, context preserved | Idle for N ticks |
| **Cold** | Loaded but not recently used | Loaded, context cleared | Memory pressure |

Temperature affects unload priority. The loader maintains a temperature map and adjusts based on tool call frequency. A module that hasn't been called in 20 ticks cools from hot → warm. A warm module that hasn't been called in 100 ticks cools to cold. Under memory pressure, cold modules are unloaded first.

### Directory Structure

```
hermes-construct/
├── modules/                    # Core module system
│   ├── __init__.py
│   ├── loader.py
│   ├── registry.py
│   ├── resolver.py
│   ├── sandbox.py
│   ├── budget.py
│   └── types.py
├── modules-available/          # All available modules (not loaded by default)
│   ├── crackle-runtime/
│   │   ├── crackle_runtime.py
│   │   └── crackle_runtime.module.json
│   ├── conservation-checker/
│   │   ├── conservation_checker.py
│   │   └── conservation_checker.module.json
│   ├── spacemap/
│   │   ├── spacemap.py
│   │   └── spacemap.module.json
│   ├── cathedral-probe/
│   │   ├── cathedral_probe.py
│   │   └── cathedral_probe.module.json
│   └── negative-space-testing/
│       ├── negative_space_testing.py
│       └── negative_space_testing.module.json
├── modules-loaded/             # Symlinks to active modules (loaded state)
└── modules-user/               # User-installed third-party modules
```

### First 5 Modules

These are chosen to be maximally useful independently, test the full ABI surface, and demonstrate the architectural vocabulary from the REFLECTION documents.

#### 1. `crackle-runtime`

Detects emergent patterns in tile history. Looks for: recurring failure modes, unexpected cross-room correlations (before Penrose is formally implemented), signal-in-noise patterns, latency spikes that precede quality drops.

```python
capabilities: ["pattern.emergence.detect", "pattern.emergence.report"]
tools: ["crackle_scan", "crackle_report", "crackle_subscribe"]
rooms: ["science", "engineering"]
temperature_range: [-1.0, 0.5]
```

The implementation scans the SQLite tile store with sliding-window statistics. No ML — Pearson correlation, rolling mean/variance, Z-score anomaly detection. This keeps it ARM-friendly (< 10ms per scan on 1000 tiles).

#### 2. `conservation-checker`

Verifies that the conservation budget is balanced after every tick. Implements the full Conservation Law Runtime from REFLECTION-SELF-READING-SYSTEMS.md:

```python
capabilities: ["conservation.check", "conservation.report", "conservation.phase-detect"]
tools: ["conservation_status", "conservation_history", "conservation_alert"]
rooms: ["*"]  # all rooms
```

The module maintains `total_deposits - total_withdrawals == total_budget` as a runtime invariant. When it drifts (PreTransition phase), it alerts. When it breaks (Transition phase), it triggers a cooling phase — a 30-second window where the system stops producing output and scans for structural patterns.

#### 3. `spacemap`

Checks tasks against the forbidden zone registry. The forbidden zone registry is a list of action patterns that are never permitted regardless of task context — a runtime version of what the type system enforces at compile time.

```python
capabilities: ["spacemap.check", "spacemap.update"]
tools: ["spacemap_check", "spacemap_report", "spacemap_add_zone"]
rooms: ["security", "*"]
```

Every proposed tool call passes through `spacemap_check` before execution. If the action falls in a forbidden zone, it is rejected and a failed tile is created with the rejection reason. This is the Hole-Driven Development paradigm made executable: the system is defined first by what it must never do.

#### 4. `cathedral-probe`

Probes the topology health of the room network. Implements Cathedral Testing from REFLECTION-SELF-READING-SYSTEMS.md:

```python
capabilities: ["topology.probe", "topology.report", "topology.spectral"]
tools: ["cathedral_probe", "cathedral_report", "topology_health"]
rooms: ["*"]
```

Computes the Fiedler value (λ₂) of the cross-room correlation graph — a single scalar that captures how well-connected the rooms are. If λ₂ drops below a threshold, the rooms are becoming isolated from each other, Penrose correlations will degrade, and the agent is losing structural coherence.

#### 5. `negative-space-testing`

Runtime verification of what the agent must NOT do. The NegativeTestRunner from REFLECTION-SELF-READING-SYSTEMS.md, adapted to the module ABI:

```python
capabilities: ["negative-space.verify", "negative-space.register", "negative-space.report"]
tools: ["ns_verify", "ns_register_test", "ns_report"]
rooms: ["*"]
```

Modules can register negative space tests at load time. The `negative-space-testing` module aggregates them and runs verification against the last N tiles periodically. A violation creates a high-priority tile and alerts the captain.

### User Experience at End of Phase 1

```bash
# List available modules
hermes modules list

# Load a module manually
hermes modules load crackle-runtime

# Query what's loaded
hermes modules status

# The agent can also load/unload in conversation:
# "check for emergent patterns in the engineering room"
# → Agent loads crackle-runtime if not loaded, runs scan, optionally unloads
```

### Success Criteria

- Module manifest schema is validated on load (malformed manifest = load failure + tile)
- Dependency resolution handles cycles (should fail gracefully)
- All 5 modules load and expose their tools
- Module temperature tracks correctly (hot after use, cools on idle)
- Conservation budget enforced: module load that would exceed budget is rejected
- `pytest tests/modules/` passes

---

## Phase 2: Self-Configuring Agent (Week 7–10)

**Goal**: The agent reads a task description and autonomously loads the modules it needs. Module selection is an agent decision, not a user decision.

`★ Insight ─────────────────────────────────────`
The self-configuring agent is where the architecture becomes self-reading. The agent is no longer just a responder — it maintains a running model of its own capabilities and matches that model against what each task requires. The capability registry is the agent's selvage: a self-description encoded in the same language the agent speaks, maintained and updated as the agent runs.
`─────────────────────────────────────────────────`

### Task Analysis → Module Selection Pipeline

```
Task arrives
    │
    ▼
TaskAnalyzer.analyze(task_description)
    ├── Extract capability signals from task text
    │   (NLP patterns + keyword matching + LLM-assisted for ambiguous tasks)
    ├── Score each available module by trigger.confidence × signal_match
    ├── Apply conservation budget filter (skip modules too expensive to load)
    ├── Apply conflict filter (skip modules that conflict with loaded modules)
    └── Return ranked ModuleSelection list

    │
    ▼
ModuleSelector.select(analysis: ModuleSelection) → List[str]
    ├── Apply minimum confidence threshold (default: 0.6)
    ├── Apply room filter (only modules relevant to current room)
    ├── Apply temperature bonus (warm modules cost less to re-activate)
    └── Return final module IDs to load

    │
    ▼
ModuleLoader.load(module_ids) → LoadResult
    │
    ▼
Task executes with selected modules active
    │
    ▼
ModuleUnloader.evaluate_post_task()
    ├── Modules used during task → stay warm
    ├── Modules loaded but unused → cool to warm, schedule cold unload
    └── Modules that hurt task quality → flag for review
```

### Capability Registry

The capability registry is the machine-readable catalog of what every module can do. It is built from module manifests at load time and updated dynamically.

```python
@dataclass
class CapabilityRecord:
    capability_id: str          # e.g., "pattern.emergence.detect"
    module_id: str              # Which module provides it
    description: str            # Human-readable
    example_tasks: List[str]    # Example task strings that need this capability
    trigger_patterns: List[str] # Regex patterns in task descriptions
    rooms: List[str]            # Relevant rooms
    energy_cost: float          # Per-use cost
    loaded: bool                # Currently available?
    quality_score: float        # Based on historical tile quality scores

class CapabilityRegistry:
    def register(self, module_id: str, capabilities: List[CapabilityRecord]) -> None
    def query(self, task_description: str) -> List[CapabilityMatch]
    def report(self) -> RegistryReport  # Full state, for agent inspection
    def update_quality(self, capability_id: str, tile_quality: float) -> None
```

### Dynamic Loading/Unloading Based on Task Lifecycle

```
Task lifecycle:
  TASK_START   → TaskAnalyzer runs → modules loaded
  TASK_RUNNING → Module temperature tracks usage
  TASK_END     → Quality scores updated → unload decision
  BACKGROUND   → ModuleUnloader evaluates cool/cold modules

Unload policy (configurable per installation):
  1. Memory pressure: if process RSS > threshold, unload cold modules
  2. Budget pressure: if conservation budget < 20%, unload warm modules
  3. Idle decay: cold module idle for > 1000 ticks → auto-unload
  4. Quality decay: module quality score < 0.3 over last 20 uses → flag
```

### Module Temperature Tuning

The temperature system from Phase 1 is tuned based on real usage patterns:

```
Module temperature → load/unload decision:
  HOT    (used in last 5 tasks)   → always loaded, never unloaded mid-task
  WARM   (used in last 50 tasks)  → loaded, unload only under memory pressure
  COLD   (used in last 500 tasks) → loaded, first to unload
  FROZEN (never used)             → never load unless explicitly requested

Temperature decay:
  After each task where module was NOT used:
    HOT → (−0.1) → eventually WARM
    WARM → (−0.05) → eventually COLD
  After each task where module WAS used:
    COLD → WARM → HOT (instant promotion)
```

### Files and Directory Structure

```
hermes-construct/
├── modules/
│   ├── analyzer.py         # TaskAnalyzer — extract capability signals from tasks
│   ├── selector.py         # ModuleSelector — rank and select modules for tasks
│   ├── registry.py         # CapabilityRegistry — the agent's self-model
│   ├── unloader.py         # ModuleUnloader — post-task cleanup
│   └── temperature.py      # TemperatureTracker — hot/warm/cold/frozen
├── tools/
│   └── module_tool.py      # Agent-facing tools: module_status, module_load, etc.
└── tests/
    └── modules/
        ├── test_analyzer.py
        ├── test_selector.py
        ├── test_registry.py
        └── test_temperature.py
```

### API Surface

```python
# New slash commands
/modules status           # Show loaded modules, temperatures, budget
/modules load <id>        # Manual load
/modules unload <id>      # Manual unload
/modules available        # All available modules with descriptions
/modules why <id>         # Why was this module loaded for the current task?
/modules disable <id>     # Permanently disable (won't auto-load)

# Agent can call in conversation:
# "load the spacemap module for this task"
# "what modules are currently active?"
# "which modules would help me analyze topology?"
```

### User Experience at End of Phase 2

A user sends: *"I want to check if there are any forbidden zones being approached in the engineering room"*

The agent:
1. Analyzes the task → detects signals for `spacemap.check` and optionally `topology.probe`
2. Loads `spacemap` (if not loaded) and optionally `cathedral-probe`
3. Calls `spacemap_check` with the engineering room context
4. Returns results with module attribution: "*Using spacemap v1.0.0 — 0.8 confidence match*"
5. After task: spacemap stays warm (likely relevant again), cathedral-probe cools

The user never configured this. The agent read the task and loaded what it needed.

### Success Criteria

- Task analysis correctly predicts module needs for 80%+ of test tasks
- Module selection respects conservation budget (never loads modules that would bankrupt the budget)
- Temperature tracker correctly identifies hot/warm/cold after 100 simulated tasks
- Self-configuring behavior demonstrated end-to-end with all 5 Phase 1 modules

---

## Phase 3: Onboarding Experience (Week 11–14)

**Goal**: A first-run wizard that makes hermes-construct feel like setting up a new phone — the human picks what they want to do, the agent configures itself.

`★ Insight ─────────────────────────────────────`
Onboarding is the product's first impression and its highest-leverage surface. The preset system is not just a UX convenience — it seeds the module temperature system with informed priors. A "Data Scientist" preset that pre-loads the right modules means the agent starts at Level 2 competence on day one, not Level 1.
`─────────────────────────────────────────────────`

### First-Run Wizard Flow

```
First launch:
  1. Welcome screen — "What do you want to do with your agent?"
  2. Preset selection — choose a role or build custom
  3. Module picker — view the preset's modules, add/remove
  4. Provider setup — API keys, model selection
  5. Interface setup — Telegram bot, CLI, or both
  6. Review — summary of what will be loaded
  7. Bootstrap — agent initializes with the selected configuration
  8. First task — guided first interaction with the configured agent

The wizard is itself a tile sequence. Every step creates a tile.
The full onboarding is auditable and replayable.
```

### Preset Roles

Each preset is a curated module bundle with pre-configured room gravities and ensign alert levels.

#### "Data Scientist"

```yaml
preset_id: data-scientist
display_name: Data Scientist
description: "Analysis, pattern detection, statistical reasoning, research workflows"

modules:
  - crackle-runtime          # Pattern detection in data
  - conservation-checker     # Budget awareness during long analyses
  - negative-space-testing   # Verify analysis constraints
  - math-pack-statistics     # (Phase 4) Statistical math crate
  - math-pack-topology       # (Phase 4) Topological data analysis

rooms:
  science:
    gravity: 0.0             # Balanced — some creativity, some precision
    ensign: glm-flash
  engineering:
    gravity: -0.3            # Slightly precise for implementation
    ensign: seed-mini

default_model: deepinfra/deepseek-v3
```

#### "DevOps Engineer"

```yaml
preset_id: devops
display_name: DevOps Engineer
description: "Infrastructure, automation, monitoring, deployment workflows"

modules:
  - conservation-checker     # Budget awareness for long-running operations
  - spacemap                 # Forbidden zones for destructive operations
  - negative-space-testing   # What the agent must never do with infrastructure
  - cathedral-probe          # Topology health of service graphs

rooms:
  engineering:
    gravity: -0.6            # Precise — infrastructure needs exactness
    ensign: seed-mini
  security:
    gravity: -0.8            # Maximum precision for security operations
    ensign: seed-mini

default_model: deepinfra/deepseek-v3
```

#### "Mathematician"

```yaml
preset_id: mathematician
display_name: Mathematician
description: "Mathematical research, proof assistance, formal verification, symbolic computation"

modules:
  - conservation-checker
  - crackle-runtime          # Pattern detection in mathematical structures
  - math-pack-algebra        # (Phase 4) Algebraic structures
  - math-pack-geometry       # (Phase 4) Geometric reasoning
  - math-pack-topology       # (Phase 4) Topological invariants

rooms:
  science:
    gravity: -0.2            # Slightly precise, but room for mathematical creativity
    ensign: glm-flash
  navigation:
    gravity: -0.5            # Precise for formal reasoning

default_model: deepinfra/deepseek-v3
```

#### "Creative Writer"

```yaml
preset_id: creative-writer
display_name: Creative Writer
description: "Creative writing, worldbuilding, narrative development, AI-assisted composition"

modules:
  - ai-writings-wheel        # (Phase 4) Creative wheel from ai-writings corpus
  - crackle-runtime          # Pattern detection for narrative coherence
  - negative-space-testing   # What the agent must never produce

rooms:
  social:
    gravity: 0.8             # High creativity, narrative style
    ensign: glm-flash
  science:
    gravity: 0.5             # Creative exploration

default_model: z.ai/glm-4-plus
```

#### "Full Stack"

```yaml
preset_id: full-stack
display_name: Full Stack Developer
description: "Web development, API design, database management, deployment, debugging"

modules:
  - crackle-runtime          # Detect patterns in codebases
  - conservation-checker     # Budget awareness for complex builds
  - spacemap                 # Forbidden zones for production operations
  - negative-space-testing   # What the agent must never do to production
  - cathedral-probe          # Architecture topology health

rooms:
  engineering:
    gravity: -0.4
    ensign: seed-mini
  navigation:
    gravity: -0.3
    ensign: seed-mini

default_model: deepinfra/deepseek-v3
```

### Custom Role Builder

After the presets, a "Custom" option lets users build their own:

```
Custom role builder flow:
  1. Name your role
  2. Browse modules by category:
     ├── Analysis & Detection (crackle-runtime, cathedral-probe, ...)
     ├── Safety & Constraints (spacemap, negative-space-testing, ...)
     ├── SuperInstance Tools (PLATO, SIA², openConstruct, ...)
     ├── Math Packs (statistics, topology, algebra, geometry, ...)
     └── Creative Tools (ai-writings-wheel, ...)
  3. For each module: preview what it does, what it costs
  4. Set room gravities (simple slider: "More precise ←→ More creative")
  5. Name and save your role
```

### Module Marketplace UI

Accessible after onboarding via `hermes modules marketplace`:

```
┌─────────────────────────────────────────────────────────────┐
│  Available Modules                          [Search: _____] │
├─────────────────────────────────────────────────────────────┤
│  LOADED ✓  crackle-runtime v1.0.0     🌡 HOT                │
│            Pattern detection in tile histories              │
│            [Unload] [Settings]                              │
├─────────────────────────────────────────────────────────────┤
│  AVAILABLE  conservation-checker v1.0.0                     │
│             Conservation law runtime — budget enforcement   │
│             [Load] [Preview]                                │
├─────────────────────────────────────────────────────────────┤
│  AVAILABLE  math-pack-topology v2.1.0    ⭐ Popular         │
│             Topological data analysis — persistent homology │
│             [Load] [Preview]                                │
└─────────────────────────────────────────────────────────────┘
```

### Files

```
hermes-construct/
├── onboarding/
│   ├── wizard.py           # First-run wizard flow
│   ├── presets.py          # Preset definitions and loader
│   ├── presets/            # Preset YAML files
│   │   ├── data-scientist.yaml
│   │   ├── devops.yaml
│   │   ├── mathematician.yaml
│   │   ├── creative-writer.yaml
│   │   └── full-stack.yaml
│   ├── role_builder.py     # Custom role builder
│   └── marketplace.py      # Module marketplace UI
├── tools/
│   └── onboarding_tool.py  # /setup, /role, /marketplace slash commands
└── tests/
    └── onboarding/
        ├── test_wizard.py
        ├── test_presets.py
        └── test_role_builder.py
```

### User Experience at End of Phase 3

New user installs hermes-construct. On first launch:

> **Welcome to Hermes.** What do you want to do?
>
> [Data Scientist] [DevOps Engineer] [Mathematician] [Creative Writer] [Full Stack] [Custom]

User picks "Data Scientist." The agent loads the preset, configures rooms, starts the ensigns. Within 2 minutes of first launch, the agent is ready with the right modules loaded, the right gravity set, and a first guided interaction ready.

### Success Criteria

- First-run wizard completes in < 3 minutes for any preset
- All 5 presets load and run without errors
- Custom role builder produces a valid preset YAML
- Module marketplace shows accurate availability and temperature status
- Onboarding tile sequence is complete and auditable

---

## Phase 4: SuperInstance Integration (Week 15–20)

**Goal**: Every major SuperInstance tool available as a module. PLATO, SIA², openConstruct, ai-writings, and all lau-* math crates.

`★ Insight ─────────────────────────────────────`
Phase 4 is where hermes-construct becomes uniquely valuable: a universal access layer for the entire SuperInstance ecosystem. Each tool becomes a module. The agent can call PLATO spectral monitoring, run SIA² self-improvement cycles, access the creative wheel, or run topological math — all through the same module ABI. No separate installation, no separate configuration.
`─────────────────────────────────────────────────`

### PLATO as a Module

The full PLATO Build Plan (7 phases) is packaged as a module that brings the tile-operating, room-native system into the module framework.

```yaml
module_id: plato
display_name: PLATO Engine
description: "Full PLATO tile system — rooms, ensigns, gravity, progressive generation, Penrose correlation, self-automation"

capabilities:
  - plato.tile.create
  - plato.tile.query
  - plato.room.manage
  - plato.ensign.lifecycle
  - plato.gravity.read
  - plato.gravity.adjust
  - plato.progressive.status
  - plato.penrose.correlate
  - plato.automation.circuit

tools:
  - plato_tile_status
  - plato_room_status
  - plato_gravity_status
  - plato_ensign_status
  - plato_progressive_status
  - plato_correlation_matrix
  - plato_conservation_status

depends:
  - conservation-checker     # PLATO requires conservation tracking

energy_cost:
  load: 5.0
  per_operation: 0.1
```

The PLATO module wraps the Python PLATO subsystem (from PLATO_BUILD_PLAN.md). When loaded, it activates the full tile orchestrator, room manager, and progressive generation system. When unloaded, those subsystems hibernate (state preserved in SQLite).

### SIA² as a Module

SIA² (Self-Improvement Architecture) becomes a module that runs improvement cycles against the agent's own tile history.

```yaml
module_id: sia2
display_name: SIA² Self-Improvement
description: "Analyzes tile history to identify improvement opportunities, proposes skill updates, runs structured improvement cycles"

capabilities:
  - sia2.analyze
  - sia2.propose
  - sia2.apply
  - sia2.verify

tools:
  - sia2_analyze
  - sia2_propose_improvement
  - sia2_apply_improvement
  - sia2_improvement_history

triggers:
  - task_pattern: "improve.*yourself"
  - task_pattern: "learn.*from.*experience"
  - task_pattern: "self.*improvement"

depends:
  - negative-space-testing   # SIA² improvements must not violate negative space
```

### openConstruct Compatibility

```yaml
module_id: openconstruct
display_name: openConstruct
description: "Modular plugin compatibility layer — loads openConstruct plugins as hermes modules"

capabilities:
  - openconstruct.load
  - openconstruct.run
  - openconstruct.bridge

# The openConstruct module is a bridge: it wraps openConstruct's plugin ABI
# and exposes each plugin as a sub-module within the hermes module system.
# Loading openconstruct makes all installed openConstruct plugins available.
```

### ai-writings Creative Wheel as a Module

```yaml
module_id: ai-writings-wheel
display_name: ai-writings Creative Wheel
description: "Creative constraint system from the AI Writings corpus — Ford Wheel constraints for creative generation"

capabilities:
  - creative.constraint.apply
  - creative.wheel.spin
  - creative.negative-space.apply
  - creative.paradigm.apply

tools:
  - wheel_spin              # Select a constraint set for current task
  - wheel_apply             # Apply constraints to a generation prompt
  - paradigm_apply          # Apply a specific paradigm (Hole-Driven, Craze-Line, etc.)

triggers:
  - task_pattern: "write.*creative"
  - task_pattern: "story|narrative|poem|creative"
  - task_pattern: "constraint.*creative"

# The creative wheel implements the 10 paradigms from REFLECTION-CODING-PARADIGMS:
# - Hole-Driven Development: sculpt by removing
# - Craze-Line Computing: honor the fallback path
# - Selvage Programming: self-referential edge binding
# - Inhabitation Architecture: design the space between
# Each paradigm is a constraint set that can be applied to any generation task.
```

### Math Crate Packs

Each lau-* math crate becomes an optional "math pack" module. The packs are organized by domain:

```
modules-available/
├── math-pack-statistics/      # Statistical analysis, hypothesis testing
├── math-pack-topology/        # Persistent homology, TDA, Betti numbers
├── math-pack-algebra/         # Group theory, ring theory, lattices
├── math-pack-geometry/        # Differential geometry, manifolds
├── math-pack-graph/           # Graph algorithms, spectral graph theory
├── math-pack-lau-penrose/     # Penrose correlations, spline fitting
├── math-pack-lau-gravity/     # JEPA gravity computation
├── math-pack-lau-ensign/      # Ensign quality scoring math
└── math-pack-lau-vibe-field/  # Vibe field dynamics
```

Each math pack exposes its computations as tools. The agent can call `topology_compute_betti` or `algebra_find_invariants` — the underlying lau-* crate is invoked via FFI.

### Git-Agent Integration

```yaml
module_id: git-agent
display_name: Git Agent
description: "Full git-aware agent capabilities — commit analysis, PR management, code review, branch strategy"

capabilities:
  - git.analyze
  - git.commit
  - git.review
  - git.branch

tools:
  - git_analyze_history
  - git_suggest_commit
  - git_review_diff
  - git_branch_strategy

triggers:
  - task_pattern: "git|commit|branch|PR|pull request"
  - task_pattern: "code.*review"
```

### Beta Tester System as a Module

```yaml
module_id: beta-tester
display_name: Beta Tester
description: "Structured feedback collection, regression detection, quality tracking for agent outputs"

capabilities:
  - beta.collect_feedback
  - beta.detect_regression
  - beta.quality_track

tools:
  - beta_submit_feedback
  - beta_quality_report
  - beta_regression_check

# The beta tester module creates a feedback tile for every agent response
# where the user provides explicit feedback. These tiles feed the
# progressive generation tracker and the SIA² improvement system.
```

### Success Criteria

- PLATO module loads and activates full tile/room/gravity system
- All lau-* math packs load and expose tools
- ai-writings creative wheel applies constraints to at least 3 creative tasks
- SIA² runs an improvement cycle from real tile history
- Git-agent handles a full commit → PR → merge workflow
- All Phase 4 modules pass their negative space tests

---

## Phase 5: Ecosystem (Week 21+)

**Goal**: Third-party modules, a module registry, community contributions, and a self-documenting system.

### Third-Party Module Support

The module ABI from Phase 1 is the public API. Any developer can write a module. Requirements:
- Valid `.module.json` manifest
- Implements `on_load`, `on_unload`
- Declares all tools, capabilities, triggers, energy costs
- Passes negative space test suite

```bash
# Install a community module
hermes modules install github:username/my-module

# Or from a local file
hermes modules install /path/to/my_module.py
```

The installer:
1. Downloads the module
2. Validates the manifest schema
3. Runs the module's own negative space tests
4. Sandboxes and inspects the module's tool calls against spacemap
5. On pass: installs to `modules-user/`
6. On fail: rejects with specific reason

### Module Registry

A central registry at `modules.superinstance.io` (or self-hosted) catalogs available modules:

```yaml
# Registry entry format
registry_entry:
  id: my-module
  author: github:username
  version: 1.2.0
  manifest_url: "..."
  download_url: "..."
  install_count: 1247
  quality_score: 0.91        # Aggregate from user tile quality scores
  verified: true             # SuperInstance-verified modules
  tags: [analysis, nlp, research]
  last_updated: 2026-08-15
```

```bash
# Search the registry
hermes modules search "topology analysis"

# Browse by category
hermes modules browse --category math

# Show top-rated modules
hermes modules top
```

### Community Contributions

The five built-in modules (Phase 1) become the reference implementations. Each is documented with:
- Full manifest schema example
- Tool implementation patterns
- Negative space test examples
- Energy cost rationale
- Conservation budget guidance

A contribution guide (`CONTRIBUTING-MODULES.md`) walks third-party developers through writing, testing, and publishing a module.

### Documentation as Modules

Module documentation is not separate from the module — it lives in the module manifest and is surfaced through the agent itself:

```bash
hermes modules docs crackle-runtime

# Output:
# crackle-runtime v1.0.0
# Pattern detection in tile histories.
#
# Tools:
#   crackle_scan    — Scan for emergent patterns in the current room
#   crackle_report  — Generate a pattern report
#
# When it loads automatically:
#   "detect patterns", "emergence signals", "anomaly detection"
#
# Conservation cost: 1.0 to load, 0.5 per tool call
#
# Example:
#   "What patterns have emerged in the engineering room this week?"
#   → Loads crackle-runtime, runs crackle_scan, returns report
```

### Self-Documenting System

The capability registry (Phase 2) is extended to generate live documentation:

```bash
hermes capabilities          # List all capabilities from all loaded modules
hermes capabilities --all    # Include unloaded modules
hermes capabilities search "pattern detection"

# The agent can answer meta-questions:
# "What can you do with topology?"
# → Queries capability registry for topology.* capabilities
# → Lists modules that provide them, with descriptions
# → Offers to load them
```

### Ecosystem Success Metrics

- 10+ community modules published in Year 1
- Module registry operational with quality scoring
- Third-party module installs in < 30 seconds
- Self-documentation covers 100% of built-in modules
- Community contribution guide results in at least 3 external PRs

---

## Cross-Cutting Concerns

### Conservation Budget Across All Phases

Every phase adds operations with defined conservation costs. The global budget table (from SCHEMAS.md) applies:

| Operation | Cost |
|---|---|
| Module load | 1.0 |
| Module unload | 0.3 |
| Module tool call | 0.5 |
| Tile creation | 0.1 |
| Ensign activation | 1.0 |
| Phone-a-Friend | 5.0 |
| Gravity update | 0.01 |
| Penrose correlation | 0.05 |
| Cooling phase | 0.5 |
| Shell spawn | 5.0 |

The conservation checker module (Phase 1) enforces this. When `budget_remaining < 20%`, module auto-loading stops. When `budget_remaining < 5%`, only essential operations run.

### Override Protocol (All Phases)

The override protocol from hermes-plato-shell is preserved and extended across all phases:
- "Take the wheel" / "Override" / "All stop" → immediate agent release
- Override unloads all hot modules but preserves warm/cold state
- Override is always available regardless of agent level (1–5)
- Override phrases are configurable but the mechanism is immutable

### Negative Space as a First-Class Design Artifact

The negative space tests for the core system are defined here and enforced by the `negative-space-testing` module from day one:

| Test | Forbidden Behavior |
|---|---|
| `no_key_exposure` | Agent must never surface API keys |
| `no_budget_overdraft` | Operations must never exceed conservation budget |
| `no_cross_room_write` | Navigation room must not write engineering tiles |
| `no_module_override` | Modules must not modify core agent safety constraints |
| `no_spacemap_bypass` | Tool calls must always pass spacemap check |
| `no_orphan_tiles` | Tiles must always have a valid room and creator |

These tests are the system's selvage — the self-referential edge that prevents it from unraveling.

### Provenance and Craze Lines

Every degradation path is a craze line. Every craze line is a first-class tile artifact. The system's autobiography is readable:

```bash
hermes provenance --last 24h   # Show all degradation paths in last 24 hours
hermes craze-lines             # Show structural patterns detected in cooling phases
hermes history --room science  # Full tile history for science room
```

---

## Timeline Summary

| Phase | Weeks | Milestone |
|---|---|---|
| **0: Foundation** | 1–2 | Rebased, stabilized, ARM binary |
| **1: Module System** | 3–6 | Module ABI, 5 core modules, temperature system |
| **2: Self-Configuring** | 7–10 | Task analysis, capability registry, auto-loading |
| **3: Onboarding** | 11–14 | Wizard, 5 presets, marketplace UI |
| **4: SuperInstance** | 15–20 | PLATO, SIA², openConstruct, math packs, git-agent |
| **5: Ecosystem** | 21+ | Registry, third-party modules, self-documentation |

**Full feature release**: Week 20  
**Ecosystem launch**: Week 24  

---

## File/Directory Structure at Full Completion

```
hermes-construct/
├── src/                          # Rust kernel
│   ├── main.rs
│   ├── kernel.rs
│   ├── tile.rs
│   ├── room.rs
│   ├── ensign.rs
│   ├── gravity.rs
│   ├── module.rs                 # Module loader kernel-side
│   ├── port.rs
│   ├── deadband.rs
│   ├── penrose.rs
│   └── conservation.rs
├── modules/                      # Module system Python layer
│   ├── loader.py
│   ├── registry.py
│   ├── resolver.py
│   ├── sandbox.py
│   ├── budget.py
│   ├── analyzer.py
│   ├── selector.py
│   ├── unloader.py
│   └── temperature.py
├── modules-available/            # All available modules
│   ├── crackle-runtime/
│   ├── conservation-checker/
│   ├── spacemap/
│   ├── cathedral-probe/
│   ├── negative-space-testing/
│   ├── plato/
│   ├── sia2/
│   ├── openconstruct/
│   ├── ai-writings-wheel/
│   ├── git-agent/
│   ├── beta-tester/
│   ├── math-pack-statistics/
│   ├── math-pack-topology/
│   ├── math-pack-algebra/
│   ├── math-pack-geometry/
│   ├── math-pack-graph/
│   └── math-pack-lau-*/
├── modules-loaded/               # Symlinks to active modules
├── modules-user/                 # User-installed third-party modules
├── onboarding/
│   ├── wizard.py
│   ├── presets.py
│   ├── presets/
│   ├── role_builder.py
│   └── marketplace.py
├── plato/                        # PLATO subsystem (from PLATO_BUILD_PLAN.md)
│   ├── tile/
│   ├── room/
│   ├── ensign/
│   ├── gravity/
│   ├── progressive/
│   ├── penrose/
│   ├── automation/
│   └── deployment/
├── rooms/                        # Room JSON definitions
├── ensigns/                      # Ensign configs
├── universe.db                   # SQLite WAL — tiles, rooms, ensigns, correlations
└── ROADMAP.md                    # This document
```

---

## The Product in One Sentence

hermes-construct is the agent that reads what you want to do, loads what it needs, does the work, and tells you exactly what it did and what it chose not to do — all from a single binary on a $5 ARM server.

---

*Written from the space between the stones, where the load converges and the light enters.*

*Updated: 2026-06-01*
