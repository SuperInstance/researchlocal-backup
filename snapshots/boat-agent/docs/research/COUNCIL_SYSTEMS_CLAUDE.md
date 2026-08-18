# COUNCIL SYSTEMS: Top 5 SuperInstance Repos for CONTROL Intelligence

**Context:** `boat-agent` kernel architecture (Rust microkernel, typed event bus, safety envelope, playbook stage-gate, shell ecology).
**Lens:** CONTROL intelligence (kernel, safety envelope, AI-authored playbooks, shell ecology, stage-gate).
**Date:** 2026-07-20
**Analyst:** Claude (systems lens, opinionated, blunt)

---

## Executive Summary

The boat-agent's CONTROL plane needs five things it doesn't fully have yet:

1. **Temporal coordination** — quorum deadlines, countdown cascades, graceful degradation
2. **LLM output governance** — bytecode enforcement, conservation laws on model behavior
3. **Shell lifecycle awareness** — molting signals, capacity-based role assignment
4. **Model health monitoring** — drift detection, quarantine, auto-degradation
5. **Intent DSL compilation** — structured shepherd language for playbooks

The ecosystem has matured since your last scan. Here are the TOP FIVE that move the needle on CONTROL.

---

## 1. `swarm-tminus` — **Temporal Spine for CONTROL**

**Mechanism:** Python PyPI package (~300 tests) providing predict-and-confirm primitives, quorum countdowns, deadline trees, and token-bucket rate limiting. File-based shared state (`.swarm/`) with heartbeat roster.

**Concrete Integration:**
- **Where:** `core/src/state/` deadline tracking, `core/src/bus/` temporal lane, `playbooks/` quorum gates
- **How:** Wrap `Predictor` around playbook proposals; `CountdownEvent` for mission phase transitions; `TokenBucket` for envelope actuation budgets
- **File plug:** `state/temporal_manager.rs` — wraps swarm-tminus primitives, emits `DeadlineExceeded` events on critical lane

**Intelligence Delta:**
- **What becomes possible:** Mission phases become time-bounded by construction. "Deploy-v1" timeout fires consensus failure → automatic rollback. No more "waiting on God" for hung playbooks.
- **New capability:** Cascading timeouts (parent 60s → child 30s) for layered control decisions. Rate-limited actuation (throttle spam) baked into envelope budget.

**Adoption Cost:** Low (stdlib-only Python, drop-in `.swarm/` convention, ~2 days Rust FFI wrapper)

**Why It Might NOT Work:**
- **Honest reason:** File-based state (JSON) works great for huts, not for 10Hz tick loops. You'll need in-memory Rust adaptation for real-time deadlines, or at least a hot cache layer. The Python `swarm-tminus` is the spec; you must implement the Rust version yourself.

---

## 2. `conservation-enforcer-rs` — **Safety Envelope 2.0**

**Mechanism:** Rust FLUX bytecode enforcement. Wraps any LLM call, validates output against conservation laws (length budget, repetition limit, category confinement, entropy floor). 150+ tests. Zero external dependencies.

**Concrete Integration:**
- **Where:** `core/src/envelope/` (new submodule: `conservation/`), `playbooks/` playbook validator
- **How:** Every playbook LLM call passes through `ConservationEnforcer::enforce_with_llm()`; violations → `IntentRejected` with structured `Violation` reason
- **File plug:** `envelope/conservation_guard.rs` — wraps playbook outputs, validates before `Intent` creation

**Intelligence Delta:**
- **What becomes possible:** Playbooks cannot silently drift into verbosity, repetition, or category violations. The envelope enforces *information* bounds, not just physical actuator bounds.
- **New capability:** "Budget decay" — each playbook invocation's conservation budget degrades by 5% per call. Prevents infinite policy loops. Violations auto-isolate sick playbooks.

**Adoption Cost:** Very Low (pure Rust, no_std-compatible, zero deps). ~3 days to integrate.

**Why It Might NOT Work:**
- **Honest reason:** FLUX bytecode is a different assembly than you're used to. You'll need to write policy in a pseudo-ASM (MOV, JMP, SYSSCALL). If your team hates low-level encoding, they'll reject it. Also: cross-impl compatibility (Python ↔ Rust) is stated but not verified by CI.

---

## 3. `hermit-crab-ecology` — **Shell Molting Signals**

**Mechanism:** 67K-word poetic spec on instance lifecycle, shell taxonomy (nerite → conch → green → murex → whelk), and molting triggers. "Sustained success is the molting signal."

**Concrete Integration:**
- **Where:** `docs/28_SHELL_ECOLOGY.md` already references it; now `core/src/state/shell_signals.rs`
- **How:** Emit `MoltingTrigger` events when agent sustained-success metrics cross threshold. Feed into mission layer's role reassignment.
- **File plug:** `state/shell_registry.rs` — tracks per-agent shell occupation, emits vacancy chain events

**Intelligence Delta:**
- **What becomes possible:** CONTROL becomes aware of *capacity constraints* as first-class events. "Vision agent growing beyond Jetson" → automatic proposal to migrate to wheelhouse (murex shell).
- **New capability:** Vacancy chain propagation — when a role molts up, the freed shell auto-triggers a "new tenant needed" event. No manual orchestration.

**Adoption Cost:** Low (this is a spec, not code). ~5 days to implement signal emission + vacancy tracking.

**Why It Might NOT Work:**
- **Honest reason:** It's 67K words of poetry. Your engineers may reject the "crab metaphor" as unserious. Also: the spec doesn't define concrete thresholds (what's "sustained success"?). You'll need to operationalize the philosophy yourself.

---

## 4. `vetcheck` — **Model Health as Veterinary Care**

**Mechanism:** Regression suites, weight drift detection, quarantine, and health certificates for LLMs. Physical exam metaphor — models are working animals that need checkups.

**Concrete Integration:**
- **Where:** `playbooks/` pre-deploy validation, `core/src/memory/model_health.rs`
- **How:** Every playbook requires health certificate before `active` gate. Periodic weight checks trigger quarantine if drift > threshold.
- **File plug:** `memory/model_veterinarian.rs` — wraps `vetcheck` API, issues `HealthCertificate`, manages `QuarantineState`

**Intelligence Delta:**
- **What becomes possible:** Models don't silently degrade. A regression (5% accuracy drop) auto-quarantines the playbook → shadow mode → human alert.
- **New capability:** "Breed-specific screening" — GPT-4 needs different tests than Claude. `VetCheck` encodes this. Pre-deployment health checks become part of stage-gate.

**Adoption Cost:** Medium (Python package, need Rust HTTP client or subprocess wrapper). ~4 days.

**Why It Might NOT Work:**
- **Honest reason:** `vetcheck` is Python; your boat-agent is Rust. You'll need to shell out or build an HTTP bridge. Also: baseline definitions are *your* problem — what's "healthy" for a trolling playbook? You must curate baselines.

---

## 5. `whistle` — **Intent DSL for Playbooks**

**Mechanism:** Declarative DSL (7-line config) compiles to PLATO rooms, conservation fences, FLUX registry entries, and rotation schedules. Replaces 500-word YAML sprawl with compiled intent.

**Concrete Integration:**
- **Where:** `playbooks/compiler.rs` (new), `playbooks/` source becomes `.whistle` files
- **How:** Shepherd writes `daily_roundup.whistle` → compiler emits `playbook.toml` + `conservation/budget_decay.bin` + `plato/room_config.json`
- **File plug:** `playbooks/whistle_compiler.rs` — invokes `whistle parse/compile`, validates references, emits to target formats

**Intelligence Delta:**
- **What becomes possible:** Playbooks become *inspectable infrastructure* rather than prose. The compiler validates breed existence, fence path, schedule syntax before runtime.
- **New capability:** "Recall triggers" become declarative (`on_drift`, `on_alarm`, `on_budget`) rather than buried in prose. Shepherds can iterate configs without touching YAML.

**Adoption Cost:** Medium (Python DSL, need to ship `whistle-dsl` or embed parser). ~6 days.

**Why It Might NOT Work:**
- **Honest reason:** Another DSL to learn? If your shepherds hate structured config, they'll reject it. Also: whistle is *new*; the compiler may have sharp edges on edge cases. You'll need to extend the grammar for vessel-specific constructs.

---

## Also-Rans (Not Top 5, But Worth Watching)

| Repo | Why Not Top 5 |
|------|----------------|
| `claw` | Ternary event classification is useful for WATCHER agents, but CONTROL already has `critical/telemetry/narrative` lanes. Adoption cost exceeds delta. |
| `t-minus-rs` | Great primitives, but `swarm-tminus` (Python) has ~300 tests and is already shipped as PyPI. Use the spec, adopt later. |
| `exocortex-rs` | Agent coordination is your problem, but `exocortex`'s memory/pubsub doesn't beat your existing `core/src/bus/`. |
| `spectro` | Multi-model cognitive spectrograph is analyst-tier (L4), not CONTROL tier. Useful for *building* playbooks, not *gating* them. |

---

## Integration Roadmap (Priority Order)

1. **`conservation-enforcer-rs`** (Week 1) — Drop into envelope, immediate safety gain
2. **`swarm-tminus`** (Week 2) — Temporal primitives for mission phase gating
3. **`vetcheck`** (Week 3) — Model health as pre-deploy gate
4. **`hermit-crab-ecology`** (Week 4) — Shell signals (operationalize the spec)
5. **`whistle`** (Week 5+) — Intent DSL (shepherd adoption curve)

---

## The Blunt Assessment

Your boat-agent kernel is solid. What it lacks is **temporal teeth** and **LLM governance**.

- `swarm-tminus` gives you deadlines that *fire* when missed — not passive timestamps.
- `conservation-enforcer-rs` gives you * bytecode enforcement* on model outputs — not wishful YAML limits.

The rest are *force multipliers*: shell ecology makes CONTROL aware of hardware; vetcheck makes models accountable; whistle makes playbooks inspectable.

**Don't adopt all five at once.** Start with conservation + temporal. Add model health when you feel the pain of silent drift. The rest is polish.

---

**Sources:**
- [swarm-tminus README](https://github.com/SuperInstance/swarm-tminus)
- [conservation-enforcer-rs README](https://github.com/SuperInstance/conservation-enforcer-rs)
- [hermit-crab-ecology README](https://github.com/SuperInstance/hermit-crab-ecology)
- [vetcheck README](https://github.com/SuperInstance/vetcheck)
- [whistle README](https://github.com/SuperInstance/whistle)
- [boat-agent Kernel Architecture](C:/Users/casey/boat-agent/docs/05_KERNEL_ARCHITECTURE.md)
- [boat-agent Shell Ecology](C:/Users/casey/boat-agent/docs/28_SHELL_ECOLOGY.md)
