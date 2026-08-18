# COUNCIL PRAGMATIST: Top 5 SuperInstance Repos for PERCEPTION + MEMORY Intelligence

**Lens:** Pragmatist — lowest adoption cost, highest intelligence delta, blunt about failure.
**Context:** `boat-agent` cascade perception (M1/M10/H1 on a laptop GPU), SQLite+CAS data twin (`VesselMemory`), git-agents roster (PULSE live), heartbeat roster, trigger mesh, scrubber UI, replay harness.
**Question asked:** Which recently-pushed SuperInstance repos could GREATLY enhance **perception** and **memory** intelligence at the LOWEST adoption cost?
**Date:** 2026-07-20
**Analyst:** Crush (pragmatist lens, blunt)
**Cross-refs:** docs/17 (cascade), docs/08 (memory), docs/27 (git-agents), `VAAS_SPECTRO_AUDIT.md`, `COUNCIL_SYSTEMS_CLAUDE.md`

---

## Executive summary

The cascade can *perceive* but it can't *disagree with itself*, can't *forget well*, can't *measure what its context truncation is costing it*, and can't *tell when a model has drifted*. Five repos fix exactly those four gaps. One is a clear winner on the math; the rest are judicious picks that trade delta for cost honestly.

The pragmatist's one-line answer: **wire `spectro` into the H1 analyst this week, `othismos-reef` into `VesselMemory` next week, and stop pretending the rest of the ecosystem matters until those two land.**

| # | Repo | Type | Δ | Conf | Cost (h) | **Score** |
|---|------|------|---|------|----------|-----------|
| 1 | `spectro` | perception | 9 | 0.80 | 8 | **0.90** |
| 2 | `othismos-reef` | memory | 8 | 0.80 | 14 | **0.46** |
| 3 | `othismos-llm` | perception | 7 | 0.60 | 12 | **0.35** |
| 4 | `exocortex-rs` | memory | 6 | 0.55 | 10 | **0.33** |
| 5 | `vetcheck` | perception (meta) | 7 | 0.70 | 18 | **0.27** |

Score = (delta × confidence) ÷ cost. Delta on a 0–10 scale; confidence 0–1; cost in engineering hours. Everything below the cut line is in *Considered and rejected*.

---

## 1. `spectro` — Multi-model cognitive spectrograph  ·  **PERCEPTION**  ·  Score 0.90

**Repo:** `SuperInstance/spectro` · Python · 42 KB · pushed 2026-07-21 (today) · pip-installable · 39/40 tests · already audited in `VAAS_SPECTRO_AUDIT.md` as production-quality.

**Mechanism (what it actually does):**
Sends one prompt to N models in parallel, then extracts (a) convergences — concepts every model returned, (b) divergences — where they disagree, (c) unique insights — what only one model saw. Output is a *spectrum*, not an answer. Explicitly **not** routing and **not** ensemble voting — it treats inter-model disagreement as signal, not noise.

**Where it plugs in (exact module):**
- **Primary:** the **H1 hourly analyst loop** (docs/17). Today H1 is one model writing one briefing. With spectro, H1 runs the day's M10 records through 3–5 models and emits *two* things instead of one: high-confidence findings (convergences → `agent.analysis.finding` with `confidence=high`) and divergence flags (→ `agent.analysis.finding` with `confidence=uncertain`, reason=`models_disagree`).
- **Concrete file:** new `core/src/perception/spectro.rs` (FFI to the Python sidecar, or a port of the ~1.4 KLOC core). M1 and M10 are left alone — spectro is too expensive for the minute loop and unnecessary for the scribe.

**Intelligence delta — what becomes possible:**
- The captain stops getting one model's confident hallucination as the daily briefing. They get a confidence *map*: "5 models agree the biomass band moved shallower; 2 models say it dissolved; here's the divergence." That is a different epistemic object.
- "Unique insight" becomes a first-class perception event — the moment one model sees something the others miss is exactly the moment a human should look. Today those are silently averaged away.
- Replay harness gets a new test shape: re-run last week's H1 across today's model flock, measure how much the spectrum shifted. That's drift detection *for free*.

**Adoption cost — 8 hours:**
- 1 h: `pip install spectro-spectrograph`, point it at DeepInfra (or local Ollama OpenAI-compatible endpoint).
- 3 h: wrap `Spectrograph.analyze()` behind a `spectro_run(prompt, models)` Rust binding (Python sidecar via `subprocess` + JSON on stdin/stdout — same shape as any cascade tool).
- 2 h: map convergences → `agent.analysis.finding` events; divergences → `agent.analysis.finding` with `provenance.class = "contested"`.
- 2 h: replay-harness test fixture + one scrubber UI panel showing the spectrum bar.

**Why it might fail (be honest):**
- **Convergence ≠ truth.** Models agree most where their training sets overlap (Wikipedia, common GitHub, same papers). On-domain echogram analysis, the models may agree for the wrong reason — they've all never seen a herring ball either. Spectro's confidence score will *look* authoritative on exactly the cases where it's least valid.
- **5× API cost per H1 run.** Acceptable hourly; ruinous if someone wires it into M1 by mistake. Hard-cap to H1 in the integration PR.

---

## 2. `othismos-reef` — Citation DAG with erosion  ·  **MEMORY**  ·  Score 0.46

**Repo:** `SuperInstance/othismos-reef` · Python · 24 KB · pushed 2026-07-17 · pip-installable · pure Python.

**Mechanism:**
A knowledge graph data structure with three gates every deposit must pass (structural integrity, connective compatibility, pressure resistance), automatic erosion of unreferenced entries (principled forgetting), and **reefquake** — removing a foundational deposit cascades structural failure through everything that cited it. Not a metaphor; a real DAG with pluggable validators.

**Where it plugs in (exact module):**
- **Primary:** `core/src/memory/mod.rs`, layered *above* `VesselMemory`. Today the memory layer "never forgets, it supersedes" (docs/08). Reef adds the missing complement: it forgets *well*, and it propagates invalidation.
- Map namespaces to deposit types: M1 notes and M10 records stay on raw `VesselMemory` (immutable observations — erosion is the wrong semantics for "what the sounder saw at 14:02"). But **playbook evidence refs, analyst findings, learned patterns, baselines** become reef deposits — those *are* knowledge claims and they *should* erode when nothing references them, and they *should* reefquake when a baseline is invalidated.
- **Concrete file:** `core/src/memory/reef.rs` — wraps `VesselMemory::put` for the `patterns`, `playbooks`, and `baselines` namespaces; periodic `reef.tick()` runs on the heartbeat roster.

**Intelligence delta — what becomes possible:**
- WATCHER's thermal baseline was wrong? Today you'd have to find every record that built on it manually. With reef: `reef.fail_deposit("watcher.baseline.alternator_v1")` → reefquake → every dependent finding is auto-flagged `provenance.class = "orphaned"`. This is the trigger mesh's missing rollback primitive.
- Erosion replaces the current ad-hoc retention schedule with a graph-aware one. A six-month-old M1 note that an active playbook still cites does *not* erode; a six-day-old note nothing references does. That is what "GC only after final read" (docs/27) actually wants.
- Gate 1 (validator) gives the kernel a place to reject malformed deposits *at write time* instead of discovering them at read time.

**Adoption cost — 14 hours:**
- 2 h: `pip install othismos-reef`, stand up a Python sidecar.
- 5 h: namespace → deposit-type mapping; write the three reef-to-VesselMemory adapters (patterns, playbooks, baselines).
- 4 h: validator callables per namespace (gate 1: schema check; gate 2: reference resolves to existing hash; gate 3: skip for v1).
- 3 h: heartbeat-tick integration for erosion + a `Reefquake` event on the bus.

**Why it might fail:**
- **Erosion is dangerous on the wrong namespace.** If you map raw M1/M10 records to deposits, the reef will quietly forget observations nobody cited — which is exactly the GC contract violation docs/27 forbids ("nothing deleted unread"). The mapping above is careful for a reason; a careless integration rots the audit trail.
- Reefquake propagation cost grows with graph density. On a vessel that's been running a year, failing one early baseline could mark thousands of descendants. Needs a depth cap or it becomes a DoS on the kernel.

---

## 3. `othismos-llm` — Context pressure gauge  ·  **PERCEPTION**  ·  Score 0.35

**Repo:** `SuperInstance/othismos-llm` · Python · 19 KB · pushed 2026-07-17 · numpy-only core, optional transformers.

**Mechanism:**
Measures the Jensen-Shannon distance between a model's next-token distribution under full context vs. truncated context. Returns a *pressure* number per token-window: high pressure = dropping these tokens materially shifts what the model will say; low pressure = safe to evict. Includes a binary-search helper (`find_safe_truncation_point`) that answers "how many tokens can you keep under 0.01 JSD?"

**Where it plugs in (exact module):**
- **Primary:** the cascade's context budget. M1 gets one frame + a gaze hint. M10 gets the canonical frame + ~10 M1 notes. H1 gets the day's M10 records. Today the eviction order is heuristic (recency, salience tags). With othismos-llm, eviction becomes *measurable*: the loop runs the analyst twice (full, truncated) at low cadence, learns which M1-note tokens are load-bearing for H1's output, and feeds that back to the M10 scribe's note-writing policy.
- **Concrete file:** new `core/src/perception/pressure.rs`. The expensive measurement runs **offline in the replay harness**, not in the hot loop — it produces a *policy* (which note shapes to keep) that M10 applies at runtime for free.

**Intelligence delta — what becomes possible:**
- The cascade stops guessing about context. "We kept the wrong 10 notes" becomes a measured quantity, not a post-hoc complaint.
- Replay harness becomes a *perception optimizer*, not just a regression suite: it can show the captain "your H1 briefing is 30% JSD-shifted by the current M10 keep-policy; here's the policy that gets you under 5%."
- Pressure numbers per note type give the scribe a training signal: "M1 notes with lat/lon are high-pressure, M1 notes with weather adjectives are low-pressure" → scribe writes more of the former.

**Adoption cost — 12 hours:**
- 2 h: `pip install othismos-llm`, run the GPT-2 concept demo to confirm the API shape.
- 6 h: **the hard part** — get next-token logprobs out of the local Ollama models the cascade actually uses (gemma4:12b, moondream-class). Ollama exposes this via the `/api/generate` `logprobs` field on recent versions; verify it works for each cascade model.
- 2 h: replay-harness integration — run each archived H1 through full vs. truncated, emit a pressure report.
- 2 h: derive a keep-policy from the report, expose it to M10 via `gaze.json`.

**Why it might fail:**
- **Local model logprob access is the real cost, not the library.** If the cascade's tiny M1 vision model doesn't expose clean next-token distributions, the gauge has nothing to measure. The README's demos use GPT-2 and toy distributions — neither proves the production path works.
- JSD on a 0.5 B iterator's next-token is noisy. The signal may only be clean on the H1 analyst (largest model), which is also the loop with the most context slack — exactly where pressure measurement matters least. There's a real risk the tool is most useful where it's hardest to run and least useful where it's easy.

---

## 4. `exocortex-rs` — Tiered memory + cortical bus  ·  **MEMORY**  ·  Score 0.33

**Repo:** `SuperInstance/exocortex-rs` · Rust · 79 KB · pushed 2026-07-20 · `cargo add si-exocortex` · zero deps · no_std-compatible · 50+ tests. (Twin Python repo `exocortex` exists; **use the Rust port** — see failure mode.)

**Mechanism:**
Tiered in-memory store (hot/warm/cold) with exponential half-life decay, a cortical bus (priority pub/sub), and a resonance engine that flags cross-agent knowledge overlap via cosine similarity. Conservation-law-aware `decide()` gates every operation against five conservation laws. Zero external deps; you bring your own async runtime and storage backend.

**Where it plugs in (exact module):**
- **Primary:** `core/src/memory/mod.rs` — as the *hot tier* in front of `VesselMemory`. Today the SQLite+CAS twin is the only tier; every recall hits disk. Exocortex-rs sits in front: hot tier = the working set the cascade is actively reading (this hour's M1 notes), warm = recent M10 records, cold = spill to VesselMemory.
- **Secondary:** the cortical bus is a candidate replacement for one lane of `core/src/bus/lanes.rs` when the message is *memory-coherent* (e.g. "PULSE saw a ramp pattern" → resonance match against WATCHER's thermal anomaly → both agents notified). The typed event bus stays for control; exocortex's bus handles the memory-resonance lane.
- **Concrete file:** `core/src/memory/hot_tier.rs` wrapping `si-exocortex::memory::MemoryStore`.

**Intelligence delta — what becomes possible:**
- Cross-agent resonance is the genuinely new organ. Today PULSE and WATCHER don't know they're seeing correlated events unless the trigger mesh is manually wired. Resonance makes the *overlap itself* an event — "your wind-shift pattern from 14:00 cosines 0.92 to your alternator-thermal anomaly from 14:02." That's docs/27's trigger mesh, auto-discovered instead of hand-authored.
- Hot tier cuts disk reads on the cascade's inner loop. Measurable latency win on M1.
- Conservation-law `decide()` is a useful pattern (not a dependency) for the safety envelope — each model invocation costs energy and information budget; exocortex-rs shows what that looks like in Rust.

**Adoption cost — 10 hours:**
- 1 h: `cargo add si-exocortex`, build the hello-world `AgentSpace`.
- 4 h: adapter from exocortex's `MemoryStore` to `VesselMemory` as the cold tier.
- 3 h: subscribe PULSE + WATCHER + SCOUT to a shared `AgentSpace`, wire one resonance event to the bus as a trigger mesh auto-source.
- 2 h: tests + a replay-harness fixture showing a resonance hit on archived data.

**Why it might fail (read the Python twin's README carefully):**
- **The Python `exocortex` README admits the embedding layer returns RANDOM unit vectors.** "Recall is by random-vector similarity, not semantic search." Resonance runs on the same embeddings. The Rust port inherits the same spec. Until you swap in real embeddings (a separate project, +20 h minimum to pick a model and wire it), *resonance is fiction* and tiered recall is keyword-shaped, not semantic.
- Adopting the Python twin by mistake is the easy way to fail — same name, misleading component table, random embeddings. **Rust port only.**

---

## 5. `vetcheck` — Model drift monitoring  ·  **PERCEPTION (meta)**  ·  Score 0.27

**Repo:** `SuperInstance/vetcheck` · Python · 44 KB · pushed 2026-07-20 · `pip install vetcheck` · passing tests.

**Mechanism:**
Three exams and two lifecycle ops, framed as veterinary care for working-animal models. **Physical exam** = regression suite (vitals: edge cases = temperature, latency = heart rate, load handling = blood pressure, function-calling = reflexes). **Weight check** = output-distribution drift via embedding distance to baseline. **Quarantine** = auto-isolate sick model + route traffic elsewhere. **Health certificate** = signed attestation with expiry.

**Where it plugs in (exact module):**
- **Primary:** new `core/src/envelope/model_health.rs` — wraps every model the cascade uses (M1 vision, M10 scribe, H1 analyst). Physical exam runs in the replay harness nightly. Weight check runs continuously on live outputs. Quarantine emits a `model.drift` event on the bus → demotes the model in the cascade's tier router → promotes a fallback.
- **Concrete integration:** the cascade already has fallback lanes (cloud when local is busy, docs/27). Vetcheck gives those lanes a *reason to fire* that isn't just "model is slow." Drift is a more important signal than latency.

**Intelligence delta — what becomes possible:**
- The cascade's perception IS the models. If gemma4:12b silently degrades after an Ollama update, today the entire M10 record is corrupt for a week and nobody knows until the captain reads a bad briefing. Vetcheck makes that a same-day event.
- "Quarantine" is the missing primitive for the cascade's model tier. Today demotion is manual. Vetcheck automates it with a paper trail (health certificate with expiry, signed).
- Replay harness gains a *longitudinal* axis: same input, model over time. Drift becomes visible.

**Adoption cost — 18 hours:**
- 2 h: `pip install vetcheck`, run the demo `physical_exam()`.
- 10 h: **the real cost** — author a regression suite per model role. M1's vitals are easy ("does it still detect a frame has a fish?"). M10's are medium ("does the scribe still emit valid JSON with lat/lon?"). H1's are hard ("is this briefing still *good*?") — likely needs a held-out golden set the captain approves once.
- 4 h: weight-check baseline capture + drift event on the bus.
- 2 h: quarantine → tier-router demotion wiring.

**Why it might fail:**
- **Defining "healthy" for an open-ended analyst is genuinely hard.** Vetcheck's pattern fits function-calling and structured output (reflexes) beautifully. It fits "describe what's on this echogram" poorly — the regression suite either becomes a brittle keyword check (false alarms) or a second LLM judging the first (expensive, and *that* judge drifts too).
- The hardest model to monitor (H1) is the most important one to monitor. Expect the cost above to be optimistic; budget 25 h.

---

## Ranking justification (the pragmatist's math)

```
                delta × confidence
  score = ─────────────────────────
                   cost (hours)
```

- **spectro** wins by a mile because the delta is large *and* the cost is genuinely 8 hours *and* the code is already audited as real. It is the only candidate where the main risk is conceptual (convergence ≠ truth), not operational.
- **othismos-reef** is second because reefquake is the trigger mesh's missing half and erosion is what docs/08's "supersede" pattern needs to stay honest. The risk is purely in getting the namespace mapping right.
- **othismos-llm** and **exocortex-rs** are tied within noise. othismos-llm ranks higher because it creates a *new measurement*; exocortex-rs is held back by the random-embedding asterisk on its headline feature.
- **vetcheck** is last of the five because its cost is dominated by *content authoring* (the golden sets), not integration. The library is fine; the work it forces you to do is the cost.

---

## Considered and rejected (one line each)

- **`agent-loop`** — Right idea (VRAM-budgeted model flock), wrong language (Python, 1.8 KLOC, hardcoded RTX 4050/4060). Extract `model_router.py`'s *ideas* into a Rust `cascade/router.rs`; do not adopt the runtime. Would rank ~0.16.
- **`chart-room`** — Conceptually a 4-panel fixed subset of spectro. The "Native" (negative-space) navigator is the one genuinely novel piece; if you want it, fork *that prompt* into spectro's ensemble, don't adopt the library. Redundant with #1.
- **`A2A-native-notebookLM`** — 10.5 MB TypeScript + SurrealDB + Next.js + LangGraph. Real software, wrong tier. This is a shoreside research librarian, not a perception/memory organ on a laptop on a boat. Revisit if the vessel ever gets a server rack.
- **`SmartCRDT`** — 7.2 MB TypeScript CRDT monorepo. About distributed state sync, not perception/memory intelligence per se. Useful for the cloud twin (docs/19), not for this question.
- **`exocortex` (Python)** — Rejected in favor of `exocortex-rs`. Same spec, but the Python twin's README admits random embeddings; the Rust port is at least honest about being a scaffold you build on.
- **`CognitiveEngine`** — 430 KB "core cognitive processing engine." Description is too vague to score; "5-level abstraction backend" without a README that says what the five levels *do* is a research time-sink, not an adoption candidate.
- **`polln`** — 35 MB TypeScript spreadsheet visualization. Wrong shape entirely.
- **`whistle`** — Intent DSL. Already covered under the CONTROL lens (`COUNCIL_SYSTEMS_CLAUDE.md`); not perception/memory.
- **`a2ui`** — Adaptive UI layer. Presentation, not intelligence.
- **`PersonalLog`** — 9.4 MB TypeScript generic logging app. Heavy, generic, no perception/memory delta.
- **`flux-showcase`** — Bytecode compiler explorer. Developer tool, not runtime intelligence.
- **`edge-weight`** — 5 KB of Cloudflare-edge thresholds. Wrong tier (edge, not vessel).

---

## Bottom line

Do these two this month, in this order:

1. **`spectro` into H1** (one day). Biggest delta, smallest cost, already audited.
2. **`othismos-reef` into the `patterns`/`playbooks`/`baselines` namespaces of `VesselMemory`** (two days). Gives the trigger mesh its missing rollback and gives docs/08's "supersede" pattern the forgetting it needs.

Then decide between **othismos-llm** (if local logprob access works on your cascade models) and **vetcheck** (if you can stomach authoring golden sets for H1). Both are real; pick the one whose failure mode you can live with.

**`exocortex-rs`** is worth a one-day spike to see if cross-agent resonance is worth pursuing *after* you swap in real embeddings. Until then, treat its headline feature as a TODO, not a capability.

Everything else on the candidate list is either the wrong tier, the wrong language, redundant with the picks above, or a heavier version of something a five-line Rust adapter can give you for free.

---

**Next:** if the captain agrees with the top two, the natural follow-up docs are `docs/29_SPECTRO_H1_INTEGRATION.md` and `docs/30_REEF_MEMORY_GATE.md`. Both can be drafted from this report without further research.
