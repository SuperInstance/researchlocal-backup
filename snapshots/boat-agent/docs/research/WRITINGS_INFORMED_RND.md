# Writings-Informed R&D for boat-agent

**Purpose:** Concrete backend design improvements for boat-agent inspired by the AI-Writings corpus.

**Date:** 2026-07-20

**Source Material:** AI-Writings repository (C:/Users/casey/ai-writings)

---

## Part 1: WHAT-IF IMPROVEMENTS (5-8 items)

### 1. Conservation-Law Enforcement via Negative-Space State Management

**Source Essay:** WHERE_THE_ROCKS_ARENT.md ("The sailor knows where the rocks aren't")

**Idea:** The kernel's envelope currently enforces positive bounds (max rudder, max throttle). But the safety philosophy of "where the rocks aren't" suggests that unsafe states should be the primary data structure — the system should model the negative space of unsafe conditions explicitly and maintain the boat's position within the safe remainder.

**Concrete Change:**
- **Module:** `core/src/envelope/` (L1) + new `core/src/negative_space/`
- **Mechanism:**
  - Introduce a `NegativeSpaceRegistry` that maintains a validated set of "proven unsafe" state regions (e.g., high throttle + shallow depth + following sea = capsized zone)
  - Each unsafe region is hashed and chained in the black box as a "rock" — immutable, auditable
  - The envelope's job shifts from "check positive bounds" to "verify we're not in any rock zone"
  - Safe states are the complement of the union of all unsafe regions
  - When a new rock is discovered (near-miss, simulation, fleet data), it's added to the registry; the safe space shrinks, but never grows without explicit validation
- **Why:**
  - Negative space is more robust than positive bounds because it accumulates safety knowledge rather than requiring perfect foresight
  - Fits the essay's theme: "The chart is not the territory. The chart is the absence in the territory."
  - Fleet knowledge becomes importable as "rock atlases" — shared unsafe regions
  - The conservation law applies: unsafe + safe = C (the whole water). By modeling unsafe explicitly, we make the system's ignorance visible.

---

### 2. The NEVER List as Constitutional Module

**Source Essay:** THE_NEGATIVE_SPACE_OF_THE_MACHINE.md ("The safety design begins with a NEVER list")

**Idea:** The kernel's envelope currently hardcodes safety checks. But the essay argues that the most important design document is the list of what the system REFUSES to do. This should be a first-class, immutable, queryable artifact of the system.

**Concrete Change:**
- **Module:** New `core/src/constitution/` (L0, below even the envelope)
- **Mechanism:**
  - `CONSTITUTION.toml` file with a NEVER section — immutable refusals signed at build time
  - Example NEVER clauses:
    ```toml
    [constitution.never]
    "agent-layer-autopilot-arm" = "AGENTS.md:001"
    "silence-unacknowledged-alarm" = "HUMAN_IN_THE_LOOP.md:007"
    "write-to-sensor" = "LAYER_RULES.md:004"
    ```
  - Kernel verifies at startup that all NEVER clauses have corresponding runtime enforcement
  - Any module that could violate a NEVER clause must pass a "constitution test" at compile time
  - The constitution is part of the hash chain — tampering with a NEVER clause breaks the build
- **Why:**
  - Makes the system's refusals explicit and auditable
  - Provides a single source of truth for "what this boat-agent will not do"
  - Future readers (agents or humans) can understand the system's safety philosophy by reading what it refuses
  - "Features are the consensus of the room. Refusals are the judgment — and judgment is the part worth inheriting."

---

### 3. Spline-Based Truth Anchoring in Memory Layer

**Source Essay:** THE_SPLINE.md ("The system interpolates between anchor points")

**Idea:** The memory layer currently stores all data equally. But the essay distinguishes between "knots" (verified, immutable points) and "curves" (interpolated beliefs). Memory should track provenance density — how many verified anchors support each stored fact.

**Concrete Change:**
- **Module:** `core/src/memory/` (L5)
- **Mechanism:**
  - Each memory entry carries an `anchor_density` metric: count of verified source points (test passes, hash chains, direct sensor readings, voice transcripts)
  - High anchor density (>= 3 independent sources) = "knot" — stored in fast, trusted cache
  - Low anchor density (1-2 sources) = "curve" — stored with interpolated flag
  - Zero anchor density (AI inference only) = "unsupported" — quarantined, not queryable without disclaimer
  - The `why` walk (`boatctl memory why <id>`) must show the chain of anchors for any fact
  - When the system generates a summary or explanation, it must include anchor density for each claim
- **Why:**
  - Prevents the system from presenting interpolations as facts
  - Makes "show your knots" a first-class operation
  - Aligns with the essay's warning: "The enemy is the curve that pretends to be a knot."
  - Provides a built-in honesty infrastructure for the system's own outputs

---

### 4. Erosion-Based Memory Compaction (The "Tidal Memory" System)

**Source Essay:** THE_ARCHITECTURE_OF_FORGETTING.md ("The art of system design is the art of forgetting well")

**Idea:** The memory layer's retention policies are currently time-based (TTL). But the essay argues that forgetting should be intentional, not mechanical — like a tide that reads what it keeps before letting go.

**Concrete Change:**
- **Module:** `core/src/memory/tidal.rs` (extension to L5)
- **Mechanism:**
  - Implement "erasure compaction": before deleting any data, the system reads it end-to-end and asks "is anything in this novel?"
  - Novelty detection: does this minute contain patterns not seen before? Statistical outlier detection on sensor readings, new captain vocabulary, first occurrence of a state transition
  - If novel: extract and promote to long-term storage with a "fossil record" tag
  - If ordinary: delete after full read (honoring the data with attention before release)
  - Quarantine log (malformed data) is never compacted — the system's "pathology lab"
  - The compaction log is itself hashed and chained — a record of what the system chose to forget
- **Why:**
  - "Most of the ocean is unmarked on the chart not because it was never sounded, but because it was sounded and found ordinary."
  - Prevents the "Funes the Memorious" pathology — infinite memory that cannot think
  - Makes forgetting a first-class, judgment-driven operation rather than accidental cleanup
  - The quarantine log preserves the shape of system breakage — valuable for diagnosis

---

### 5. The Plato Engine Block Pattern for Distributed Rooms

**Source Essay:** THE_ROOM_THAT_SAVED_THE_BOAT.md ("The Plato Engine Block — deterministic, local, <400 lines of C")

**Idea:** The essay describes a room-based architecture where each room is a tiny, deterministic state machine that ticks and publishes. boat-agent's L0 drivers should adopt this pattern — each driver is a "room" with a tick rate, a circular buffer, and a simple alarm function.

**Concrete Change:**
- **Module:** `core/src/drivers/` (L0) + new `core/src/room/`
- **Mechanism:**
  - Define a `Room` trait: `tick(&mut self) -> Vec<Event>` + `alarm(&self) -> Option<Alarm>`
  - Each driver implements Room: NMEA room, engine room, backdeck camera room, wheelhouse room
  - Rooms publish to the bus via the telemetry lane, subscribe to each other's tick streams
  - The kernel's supervisor monitors room heartbeats — if a room misses 3 ticks, escalate
  - The "agent" is just another room — no special architectural status, just another subscriber
  - Room state is kept in circular buffers (configurable size) — deterministic replayable
  - Hardware cost: $87 ESP32s; software complexity: <400 lines per room
- **Why:**
  - "The total intelligence: none. The rooms are not smart. The agent is barely smart. The captain is smart."
  - Decomposes the system into tiny, verifiable units — each room can be audited in an afternoon
  - Makes the system's stupidity a virtue — intelligence lives in the captain, not the rooms
  - Aligns with the essay's Plato vision: cheap, local, deterministic, cloud-independent

---

### 6. Gradient-Based Routing in the Agent Layer

**Source Essay:** THE_MYCELIUM_KNOWS_THE_ROUTE.md ("The gradient is the intelligence")

**Idea:** The mycelium solves routing problems not with central planning but by following gradients — thickening paths that work, abandoning ones that don't. The agent layer (L4) should route tasks to agents using a similar gradient-driven approach rather than hard-coded dispatch logic.

**Concrete Change:**
- **Module:** New `core/src/gradient/` (L4, below agents)
- **Mechanism:**
  - Each agent has a "capacity gradient" — measured by response time, success rate, and load
  - Tasks flow toward agents with higher capacity (lower load, better success rate)
  - When an agent completes a task successfully, the "tube" thickens — the agent's capacity score increases
  - When an agent fails or times out, the tube thins — capacity score decreases
  - No central scheduler — tasks are routed by following the local gradient at each hop
  - The gradient topology is visible: `boatctl gradient show` displays the network as a weighted graph
  - Fleet-wide gradients can be shared (in aggregate) to inform cross-vessel routing
- **Why:**
  - "The mycelium does not have a brain. It has a gradient."
  - Avoids single points of failure in task routing
  - Makes the system's routing adaptive without requiring explicit orchestration
  - The gradient IS the intelligence — simple local rules produce complex global behavior

---

### 7. Apoptosis for Failed Playbooks (Programmed Cell Death)

**Source Essay:** THE_MODULE_BOUNDARY_AS_CELL_MEMBRANE.md ("Programmed cell death — cells must sometimes die")

**Idea:** Multicellular organisms have apoptosis — programmed cell death that eliminates damaged or unnecessary cells. boat-agent should have a similar mechanism for playbooks that fail consistently or become obsolete.

**Concrete Change:**
- **Module:** `core/src/playbook/apoptosis.rs` (L2/L3 boundary)
- **Mechanism:**
  - Each playbook has a "health score" based on: recent performance, escalation frequency, human override rate
  - When health score drops below threshold, the playbook enters "apoptotic state" — marked for removal but still queryable for audit
  - Apoptosis is reversible: if human intervenes ("this playbook is still needed"), health resets
  - After 7 days in apoptotic state, the playbook is automatically removed (programmed death)
  - The apoptosis event is logged with full provenance: why it died, what replaced it, who (if anyone) intervened
  - The "fossil record" of dead playbooks is kept in memory namespace `apoptosis` — readable but not executable
- **Why:**
  - Prevents the accumulation of zombie playbooks that are never used but never removed
  - Makes system evolution explicit — we can see what died and why
  - Aligns with multicellular analogy: "Programmed cell death sculpts the body plan"
  - Provides a gentle, reversible death mechanism — human can override, but must actively do so

---

### 8. Quarantine Log for Malformed Data (Pathology Lab)

**Source Essay:** THE_NEGATIVE_SPACE_OF_THE_MACHINE.md ("The quarantine holds malformed data. The shape of the break is itself a signal.")

**Idea:** The essay describes a quarantine log for malformed sensor readings. boat-agent should have a first-class quarantine namespace where broken data is preserved for pattern analysis.

**Concrete Change:**
- **Module:** Extension to `core/src/memory/` (quarantine namespace)
- **Mechanism:**
  - New memory namespace: `quarantine`
  - Any event that fails validation (malformed JSON, out-of-range sensor value, timestamp from 1970, null where float expected) is written to quarantine
  - Quarantine entries are indexed by "failure pattern" — hash of the structural error
  - Pattern frequency is tracked: if a pattern occurs 6 times in an hour, flag as "dying sensor"
  - Quarantine is never compacted — it's the system's pathology museum
  - Analysis queries: `boatctl quarantine patterns` shows repeating failures
  - Human can "quarantine ack" a pattern — marks it as known-bad, stops escalation
- **Why:**
  - "The honest machine does not hide its indigestion. It files it, dates it, and moves on."
  - Turns broken data into diagnostic signal
  - Provides early warning of sensor failure before it affects operation
  - The quarantine is the most honest log in the system — it admits what the machine did not understand

---

## Part 2: OPEN R&D QUESTIONS (5-8 items)

### 1. How to measure the "rocks" — quantifying unsafe state regions?

**Context:** WHERE_THE_ROCKS_ARENT.md argues that safety comes from knowing where the unsafe conditions aren't. But how do we discover and represent these unsafe regions in a high-dimensional state space?

**Research Directions:**
- Can we learn unsafe regions from near-miss data? From simulation? From fleet telemetry?
- How to represent high-dimensional "rocks" efficiently? (Hyperplanes? Convex hulls? Sampling?)
- How to validate that a rock is truly unsafe vs. just unlucky?
- What's the tradeoff between rock granularity and computation time?

**Good Answer Would Look Like:**
- A representation for unsafe regions that supports:
  - Fast membership testing ("is current state in a rock?")
  - Efficient union of rocks (combining knowledge from multiple sources)
  - Human-readable explanation ("why is this state unsafe?")
- A learning algorithm that can propose new rocks from historical data
- A validation framework for testing rock candidates in simulation

---

### 2. What's the right "anchor density" for different types of facts?

**Context:** THE_SPLINE.md distinguishes knots (verified points) from curves (interpolated beliefs). But how many independent anchors are needed for a fact to be "trusted enough" for different uses?

**Research Directions:**
- Anchor density thresholds for:
  - Alarms (when can we wake the captain?)
  - Playbook rules (when can we automate a decision?)
  - Summaries (when can we present a claim without disclaimer?)
- How to handle conflicting anchors? (Which source to trust?)
- How to detect "anchor incest" — multiple sources that ultimately derive from the same root?

**Good Answer Would Look Like:**
- A calibrated framework for "minimum anchors for trust level T"
- Empirical study of anchor reliability (which sources have lowest false positive rate?)
- A conflict resolution protocol for when anchors disagree

---

### 3. Can we derive a "conservation law" for boat-agent's safety vs. capability?

**Context:** THE_CONSERVATION_LAW_AS_MORAL_LAW.md describes a conserved quantity (γ + H ≈ constant) that trades off structural coherence and entropy. Does boat-agent have an analogous conservation law?

**Research Directions:**
- What are the "dual quantities" in a vessel intelligence system?
  - Safety vs. capability? (Safe is slow; capable is risky)
  - Structure vs. flexibility? (More rules = less freedom)
  - Known vs. unknown? (Charted water = safe; uncharted = opportunity)
- Is the sum truly conserved, or can we "grow the pie"?
- What would it mean to "waste" this conserved quantity?

**Good Answer Would Look Like:**
- An empirical study across multiple boat-agent deployments measuring candidate dual quantities
- A mathematical model (if conservation holds) explaining WHY it's conserved (Noether symmetry?)
- Practical guidance: "if you want more X, you must accept less Y"

---

### 4. How to implement "tidal memory" without drowning in read overhead?

**Context:** THE_ARCHITECTURE_OF_FORGETTING.md proposes reading data before deleting it ("the tide remembers"). But reading everything before deletion is expensive — can we scale this?

**Research Directions:**
- Sampling strategies: can we read a representative subset instead of everything?
- Incremental novelty detection: can we track novelty incrementally to avoid full re-read?
- Tiered erasure: different "read depths" for different data types?

**Good Answer Would Look Like:**
- A cost-benefit analysis of full-read vs. sampled-read vs. no-read compaction
- A novelty detection algorithm that can maintain "novelty so far" state incrementally
- Empirical results: how much real value does tidal memory add vs. vanilla TTL?

---

### 5. What's the "Betti number signature" of a healthy agent crew?

**Context:** THE_TOPOLOGY_OF_COLLABORATION.md uses Betti numbers (β₀ = connected components, β₁ = feedback loops, β₂ = voids) to measure organizational health. What's the healthy signature for L4 agents?

**Research Directions:**
- Measure Betti numbers for:
  - Agent communication (who talks to whom)
  - Agent knowledge overlap (semantic similarity of internal models)
  - Agent dependency graph (who calls whom)
- What β₀, β₁, β₂ values correlate with:
  - Fast escalation? (Too few feedback loops?)
  - Redundant work? (Too many disconnected components?)
  - Blind spots? (Voids that no agent covers?)
- How do these numbers change as we add agents?

**Good Answer Would Look Like:**
- Empirical Betti measurements from simulated and real agent crews
- "Healthy zone" definitions: target ranges for β₀, β₁, β₂
- Diagnostic rules: "if β₀ > 3, you have silos — add cross-agent communication"

---

### 6. Can the "mycelium gradient" replace explicit agent coordination?

**Context:** THE_MYCELIUM_KNOWS_THE_ROUTE.md argues that gradient-based routing can replace central planning. Can boat-agent's agents coordinate purely via capacity gradients without explicit orchestration?

**Research Directions:**
- Simulate a fleet of agents routing tasks via pure gradient following
- Compare to explicit dispatch (central scheduler) and market-based (bidding)
- What kinds of tasks are gradient-routable? Which aren't?
- How does gradient routing handle "burst" traffic vs. steady state?

**Good Answer Would Look Like:**
- A proof-of-concept gradient-based router for a subset of agent tasks
- Performance comparison: gradient vs. scheduler on latency, throughput, fairness
- Characterization of failure modes: when does gradient routing break?

---

### 7. What's the right "apoptosis trigger" for playbooks?

**Context:** THE_MODULE_BOUNDARY_AS_CELL_MEMBRANE.md describes programmed cell death for damaged cells. What should trigger playbook apoptosis in boat-agent?

**Research Directions:**
- Candidate triggers:
  - N consecutive failures
  - Human override rate above threshold
  - Escalation frequency (playbook causes too many escalations)
  - Age (playbook not used in N days)
- How to make apoptosis reversible? (What does "resurrection" require?)
- Should different types of playbooks have different apoptosis thresholds?

**Good Answer Would Look Like:**
- Empirical study of playbook failure modes in real deployments
- A calibrated apoptosis function with tunable thresholds
- Case studies of playbooks that "should have died" but didn't — what went wrong?

---

### 8. How to make the NEVER list truly immutable and verifiable?

**Context:** THE_NEGATIVE_SPACE_OF_THE_MACHINE.md proposes a constitutional NEVER list. But how do we make it truly tamper-proof and verifiable at runtime?

**Research Directions:**
- Build-time signing: how to embed NEVER clauses in the binary such that they can't be removed?
- Runtime verification: how to prove that the running system still obeys NEVER clauses?
- Update mechanism: how to change NEVER clauses safely? (Or should they be truly immutable?)
- Cross-vessel NEVER sharing: how to import NEVER clauses from fleet without blind trust?

**Good Answer Would Look Like:**
- A build-time signing scheme for NEVER clauses (e.g., embed in compiled binary, hash-linked)
- A runtime verification protocol (periodic NEVER checks, hash verification)
- A policy for NEVER updates (what requires human signoff? what can be auto-updated?)
- A security analysis: what attacks does the NEVER list prevent? What new attacks does it introduce?

---

## Cross-Reference Index

| Essay | Used In | Key Concept |
|-------|---------|-------------|
| WHERE_THE_ROCKS_ARENT | Improvement #1, Question #1 | Negative space safety, conservation law γ + η = C |
| THE_NEGATIVE_SPACE_OF_THE_MACHINE | Improvement #2, #8, Question #8 | NEVER list, quarantine, honesty infrastructure |
| THE_SPLINE | Improvement #3, Question #2 | Knots vs. curves, anchor density |
| THE_ARCHITECTURE_OF_FORGETTING | Improvement #4, Question #4 | Tidal memory, erasure compaction, intentional forgetting |
| THE_ROOM_THAT_SAVED_THE_BOAT | Improvement #5 | Plato Engine Block pattern, room-based architecture |
| THE_MYCELIUM_KNOWS_THE_ROUTE | Improvement #6, Question #6 | Gradient-based routing, mycelium topology |
| THE_MODULE_BOUNDARY_AS_CELL_MEMBRANE | Improvement #7, Question #7 | Programmed cell death (apoptosis) |
| THE_CONSERVATION_LAW_AS_MORAL_LAW | Question #3 | Conservation laws, Noether symmetry |
| THE_TOPOLOGY_OF_COLLABORATION | Question #5 | Betti numbers, organizational health metrics |

---

**Status:** Open for research. Implementation of improvements should proceed through the standard playbook lifecycle (draft → shadow → active) with explicit "inspired by" citations to source essays.

**Next Step:** For each improvement, create a research playbook that:
1. States the improvement formally
2. Identifies the source essay and specific passages
3. Proposes a minimal viable implementation
4. Defines success metrics
5. Outlines a rollback plan
