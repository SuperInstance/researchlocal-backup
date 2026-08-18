# 29 — The Vessel Intelligence Council: What the Org Can Teach the Boat

> **Target Audience:** All agents planning capability work.
> **Purpose:** Consolidate the four-lens deep-think (2026-07-20) into
> adoption decisions: which sibling repos genuinely raise the vessel's
> intelligence, where they plug in, and in what order.
> **Status:** Governing for capability adoption · Sources:
> [systems/claude](research/COUNCIL_SYSTEMS_CLAUDE.md) ·
> [pragmatist/crush](research/COUNCIL_PRAGMATIST_CRUSH.md) ·
> [captain/mini-agent](research/COUNCIL_CAPTAIN_MINI.md) ·
> temporal/fleet lens (explore agent, inline)

---

## The question and the shape of the answer

"Which recently-pushed SuperInstance repos could greatly enhance our
intelligence as a vessel?" Four models answered through four lenses:
control intelligence, perception/memory at lowest cost, what the
captain feels, and temporal/spatial/fleet awareness. The striking part
is the **convergence**: four different lenses independently circled the
same short list, for different reasons. That convergence is the
strongest signal in this document.

## Convergent picks (multiple lenses, adopt in this order)

### 1. `spectro` — perception that can disagree with itself
**Lenses:** pragmatist (#1, score 0.90), systems (top pick), captain (noticed in week 1)
Today H1 is one model writing one confident briefing. Spectro runs the
day's records through 3–5 models and emits a **spectrum**: convergences
(high-confidence findings), divergences (marked `contested`), and
unique insights (the one model that saw what the others missed — often
the exact moment a human should look). The captain stops receiving one
model's hallucination as gospel and starts receiving an epistemic map.
Plug-in: H1 analyst only — **hard-capped** (5× cost is ruinous in M1).
Cost: ~8h. Honest risk: convergence ≠ truth — models can agree for the
wrong reason on out-of-domain sonar; label confidence by class, not by
vote count alone.

### 2. `othismos-reef` — memory that erodes with dignity
**Lenses:** pragmatist (#2), temporal (honorable mention), systems
Citation-DAG knowledge with admission gates, automatic erosion of
unreferenced entries, layer sinking, and `blast_radius` ("if this
pattern is invalidated, which playbooks fall?"). This answers Q-LEARN-1
(confidence decay) and Q-AGENT-2 (baseline drift) with a mechanism
instead of a policy. Plug-in: `VesselMemory` — adopt the blast-radius
query and erosion semantics inside `core/src/memory/` (port, don't
depend). Cost: ~14h. This is memory *governance* the docs/08 design
gestured at but never mechanized.

### 3. `signalk-bridge` — the `vessel.near()` primitive
**Lenses:** temporal (#3, "cheapest high-delta port")
Threshold detectors over the NMEA stream: course changes, speed
transitions, shoaling, thermal fronts, and **geofence arrival/departure**
— which is literally `vessel.near(island="Bold")` from docs/27's
trigger examples. Converts "we have GPS" into "the system knows when we
entered the lee zone." Port the detectors (~½ day), NOT the cloud
plumbing (it writes straight to relay — violates local-first; on-boat
it emits bus events). Thresholds live in `vessel.toml`, calibrated per
vessel, labeled `provenance.class="threshold"`.

### 4. `ship-log-search` + `ship-log-sync` — spatial memory
**Lenses:** temporal (#2)
Every entry `{text, category, ts, lat, lon}` with three query modes:
semantic, spatial (`nearby`), timeline. This is the **lee-correlation
substrate**: "what did we see within 3 km of Bold Island with NW wind
>15 kn?" — one query across months of records from every git-agent.
The sync half is already local-first and idempotent. Work: schema
alignment to bus event kinds + provenance classes. Guard: spatial/
temporal (exact) queries may feed decisions; semantic (fuzzy) may only
inform — fake confidence is the worst sin.

### 5. `swarm-tminus` — the temporal spine (already adopted; go deeper)
**Lenses:** temporal (#1)
Beyond the heartbeat/quorum extraction from docs/28: `DeadlineTree` =
escalation `fallback_on_timeout` enforcement (Q-HITL-2), `RatePair` =
value÷compute scheduling and lane backpressure, `Campaign` DAG =
multi-step missions (scout → chart consult → lee estimate → escalate).
Zero additional cost — pip-installed, file conventions already match.

### 6. `fleet-platform` + `SmartCRDT` — fleet learning, gated
**Lenses:** temporal (#4, #5)
fleet-platform is the multi-vessel cloud pre-built (D1 schema is a
ready cross-vessel data model including `training_labels`); SmartCRDT
supplies the convergence math (OR-Set for `vocab`, G-Counters for
catch/corroboration counts) so vessels syncing after a week apart
converge mathematically instead of clobbering each other's facts.
Both gated hard by the trust model: imports land as inert candidates,
confidence transfer-discounted, local replay required (Q-LEARN-2).
Adopt the math (~1–2 days/type in Rust), not the stack.

## Single-lens picks worth their mention

- **vetcheck** (pragmatist): model drift + quarantine — watch the
  distilled models (docs/28 molting) with it. Cost 18h; defer to the
  first molt.
- **othismos-llm** (pragmatist): context-truncation pressure gauge —
  when H1's context budgeting matters, this measures what truncation
  costs. Defer until briefings hit context limits.
- **a2ui** (captain lens): intent-driven ad-hoc views — "show me every
  Tuesday off the rock" without a built screen. Compatible only if
  rendered against boatctl, never direct DB (docs/10).
- **chart-room** (temporal honorable mention): four-perspective
  synthesis for SCOUT — revisit when single-pass synthesis measurably
  underperforms.

## Explicitly rejected this round (with reasons)

| Repo | Why not |
|------|---------|
| exocortex (Python) | headline "resonance" runs on random unit vectors per its own README — pattern donor only |
| A2A-native-notebookLM | SurrealDB/LangGraph/Next.js — wrong shell for a boat |
| polln | stochastic selection near actuation violates the determinism directive |
| PersonalLog | consumer shell waiting for fleet protocols — not vessel-shaped |
| CognitiveEngine | boilerplate smell, dream mode parked |
| VaaS | design fiction (per the audit) — safety-doc ideas only |

## The adoption sequence (next 4 weeks)

1. **Week 1:** spectro → H1 (8h) + signalk geofence detectors (4h).
2. **Week 2:** othismos-reef erosion + blast-radius → VesselMemory (14h).
3. **Week 3:** ship-log-search/sync spatial layer (schema alignment).
4. **Week 4:** swarm-tminus DeadlineTree into escalation timeouts +
   Campaign DAG for missions. Fleet (fleet-platform + CRDT types) waits
   for vessel #2 — the trust model must be built before the second
   boat exists, not after.

## What the council agrees is the deeper point

The vessel's intelligence doesn't grow by adding models. It grows by
**disagreement made visible** (spectro), **forgetting made principled**
(reef), **place made queryable** (ship-log), **time made structural**
(tminus), and **trust made computable** (the fleet trust model). Every
one of those is a pattern this org built elsewhere for other reasons —
the boat just turns out to be where they were all heading.

---

**Cross-references:** docs/24 (first ecosystem study), docs/27/28
(git-agents, shells), docs/13 Q-LEARN-2 / Q-SYNC-1 / Q-HITL-2,
docs/research/ (the four council reports).
