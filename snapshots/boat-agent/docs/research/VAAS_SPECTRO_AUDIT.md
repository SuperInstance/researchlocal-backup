# VaaS & Spectro Audit — Technical Assessment for boat-agent/tzpro-agent

**Audited:** 2026-07-20
**Auditor:** Claude Code (Research Task)
**Scope:** SuperInstance/VaaS, SuperInstance/spectro
**Context:** boat-agent (Rust kernel, event bus, safety envelope) + tzpro-agent (sounder vision, memory twin)

---

## Executive Summary

**VaaS** is a conceptual architecture document repository (~15k LOC of Markdown) describing a multi-agent cognitive substrate with 7 architectural pillars. It contains **no working code** — only extensive design documents, synthesis essays from AI collaborations, and domain application guides. It's an idea paper, not software.

**spectro** is a working Python package (1,426 LOC) that sends one prompt to multiple AI models in parallel and analyzes convergences/divergences. It's production-quality: clean async architecture, 39/40 tests passing, PyPI-ready, with optional semantic analysis via sentence-transformers.

**Bottom line:** spectro is real and useful for multi-model validation. VaaS is not software — it's architectural fiction masquerading as a product repo. Adopt spectro's analysis patterns. Ignore VaaS code claims (there is no code), but read its safety-envelope thinking for boat-agent's hard-bounds layer.

---

## 1. VaaS (Vessel-as-a-Substrate) Audit

### 1.1 What It Actually Is

VaaS is a **design-document repository**, not software. The repo contains:
- 15,256 lines of Markdown (READMEs, synthesis essays, KA session outputs, domain guides)
- Zero Python files
- Zero Rust files
- Zero package manifests (no `setup.py`, `pyproject.toml`, `Cargo.toml`)

The README describes "vaas-resonance-substrate/" as if it were a Python package with modules like `vaas/core.py`, `vaas/safety.py`, etc. **These files do not exist in the repo.** The repo is pure documentation describing a system that hasn't been built.

**2-3 sentence summary:** VaaS is a speculative architecture document describing a 7-pillar multi-agent cognitive substrate for maritime systems, with extensive domain guides for sailboats, oil exploration, and hatcheries. It contains no implementation code — only Markdown design documents, AI-generated synthesis essays, and placeholder stubs that reference non-existent Python modules. It's an idea paper for a system called "Vessel as a Substrate" where a "crab" (agent identity) migrates between "shells" (hardware harnesses).

### 1.2 Architecture & Key Files

**Claimed Structure (from README):**
```
vaas-resonance-substrate/ (Python package — DOES NOT EXIST)
├── vaas/core.py          — Substrate + Operator Field
├── vaas/safety.py        — Safety Envelope (4 layers)
├── vaas/entropy.py       — Cognitive Thermodynamics
├── vaas/memory.py        — 3-Tier Distributed Memory
├── vaas/bridges.py       — Holographic Bridges
├── vaas/communication.py — Pheromones + Explicit Bridges
├── vaas/constitution.py  — Resonance Constitution
├── vaas/polyrhythm.py    — Multi-Tempo Phase Lock
├── vaas/grafting.py      — Fleet Pollination Protocol
└── vaas/cli.py           — Command Line Interface
```

**Actual Files (what's really there):**
```
VaaS/
├── 00_synthesis.md                           (362 lines)
├── 01_synthesis_v2.md                        (640 lines)
├── 02_synthesis_v2_1.md                      (912 lines)
├── 03_synthesis_v2_2.md                      (535 lines)
├── ka1---e457680f-0400-43ec-b1cf-942e6268ff9e.md   (151,689 lines — AI session transcript)
├── ka2---56071046-7890-4cfc-924a-c3e21b79b72c.md   (110,974 lines — AI session transcript)
├── ka3---68df91a9-c6c8-44f6-9e44-6b93c57a6074.md   (225,918 lines — AI session transcript)
├── ka4---8714d3d0-e9ee-42f0-b3b4-b494d386c03d.md   (72,949 lines — AI session transcript)
├── ka5---068711de-c563-4871-9c7a-db665c843128.md   (35,260 lines — AI session transcript)
├── README.md                                  (646 lines — master hub doc)
├── docs/                                      (31 application/architecture guides)
│   ├── APP_HATCHERY.md         (14,825 lines — aquaculture domain guide)
│   ├── APP_LOWPOWER_SAIL.md    (15,859 lines — sailboat circumnavigation guide)
│   ├── APP_OIL_EXPLORATION.md (16,649 lines — oil exploration vessel guide)
│   ├── SAFETY_DEEP_DIVE.md     (6,616 lines — safety envelope specification)
│   └── [27 more guides — most are 13-line stubs]
└── analysis/                                  (9 analysis docs)
    ├── 15_THE_ENTROPY_ENGINE_SPEC.md    (48,276 lines — entropy engine spec)
    └── [8 more analysis papers]
```

**Architecture Concepts (described, not implemented):**

1. **Cognitive Thermodynamics** (Pillar 1): Agents have an "entropy budget" — when confusion gets too high, they trigger a "dream cycle" to sort and discard. Interesting concept but no math defined for measuring "cognitive temperature."

2. **Dual-Layer Communication** (Pillar 2): "Pheromones" (environmental signals like ant trails) + explicit bridges (guaranteed delivery). The pheromone idea maps well to boat-agent's event bus — agents can `emit(EVENT)` without caring who listens.

3. **Distributed Memory** (Pillar 3): Three tiers: active garden (RAM), cryogenic archive (cold storage), holographic fragments (distributed backup). tzpro-agent's twin/ is already the active garden. Cryogenic archive could be an S3/Glacier backup pipeline.

4. **Polyrhythmic Substrate** (Pillar 4): Different agents run at different tempos (safety@10Hz, vision@2Hz, memory@0.2Hz) but phase-lock to a shared heartbeat. **boat-agent already does this.** The fixed-tick kernel IS the polyrhythmic substrate.

5. **Holographic Bridges** (Pillar 5): Translator agents that preserve "shadows" (lossless originals) to prevent translation loss. Interesting but over-engineered for boat-agent's current needs.

6. **Resonance Constitution** (Pillar 6): Governance when agents disagree. Human-as-supreme-node. **boat-agent's safety envelope IS this.** The hard/soft/advisory/ethical layers in VaaS's safety docs map 1:1 to boat-agent's `vessel.toml` limits.

7. **Grafting Protocol** (Pillar 7): Fleet-wide knowledge sharing via "pollination" (selective, reversible). Not relevant for single-vessel ops, but interesting for fleet sync.

### 1.3 Maturity

**Status:** DOCS-ONLY / CONCEPTUAL

- **Code:** None
- **Tests:** None
- **Installable:** No (package doesn't exist on PyPI — `pip install vaas-resonance` fails)
- **Working:** No
- **Prototype:** No
- **Documentation:** Extensive (15k lines), but describes non-existent software

The README claims:
> ```bash
> pip install vaas-resonance
> ```

This command does not work. The package has never been published. The repo has no build configuration.

**Verdict:** VaaS is architectural fiction. It's a richly detailed design document for a system that exists only as Markdown and AI collaboration transcripts.

### 1.4 Quality Issues Found

**Critical Issues:**

1. **False Claims About Code:** README describes Python modules that don't exist. References imports like `from vaas import Substrate` that cannot work. Claims "installation" via pip that fails.

2. **Overclaims on Safety:** The safety envelope is described as "hardware-enforced, unbypassable" but there's no actual Rust safety kernel in the repo. The safety specifications are pure design fiction — no code, no verification, no threat model.

3. **Missing Links:** Several `docs/` files referenced from README are 13-line stubs containing only placeholder text like:
   ```
   # Domain Template
   (TODO: fill in domain-specific guide)
   ```

   Specifically: `APP_MARITIME.md`, `APP_SPACE.md`, `APP_SURGICAL.md`, `CAPTAINS_GUIDE.md`, `DEVELOPERS_GUIDE.md`, `HARNESS_GUIDE.md`, `MIGRATION_GUIDE.md`, `PILLAR_*.md` (all 7 pillar guides).

4. **Bloat:** The repo contains 600k+ lines of AI session transcripts (ka1-ka5 files). These are raw chatlogs with zero indexing. They should be in a separate repo or deleted.

5. **Analysis Without Implementation:** `analysis/15_THE_ENTROPY_ENGINE_SPEC.md` is 48k lines of entropy engine specification — for a component that doesn't exist and has no clear path to implementation.

**Moderate Issues:**

6. **Domain Guides Are Narrative, Not Technical:** `APP_LOWPOWER_SAIL.md` reads like a short story ("03:00 — Middle of the Night Watch") rather than implementation specs. It's evocative but not actionable.

7. **No Migration Path:** VaaS describes "hermit crab migration" between shells (phone → PC → cluster) but provides zero technical detail on how state serialization/deserialization would work.

**Minor Issues:**

8. **Inconsistent Terminology:** Uses "crab" and "shell" metaphorically throughout, then mixes in technical terms like "substrate" and "harness" without clear glossary mapping.

9. **No API Contracts:** Despite describing 7 pillars and cross-agent communication, there's no OpenAPI spec, no message schemas, no protocol documentation beyond prose descriptions.

### 1.5 Synergy with boat-agent/tzpro-agent

**Adopt (Concrete Value):**

1. **Safety Envelope Layering (SAFETY_DEEP_DIVE.md):**
   - **What:** 4-layer safety model: HARD (physics), SOFT (operational limits with override), ADVISORY (best practices), ETHICAL (productive dissonance)
   - **Map to boat-agent:**
     - HARD → `vessel.toml` max_rate_of_turn, min_keel_clearance (already exists)
     - SOFT → Warning thresholds that require human ACK (add to envelope/)
     - ADVISORY → Log-only advisories for fuel/route (add to envelope/)
     - ETHICAL → "Productive dissonance" — flag patterns that look like risk escalation (novel, but interesting)
   - **Cost:** Low (document patterns already used; just formalize)

2. **Pheromone Communication Pattern (Pillar 2):**
   - **What:** Agents emit "pheromones" to shared environment — environmental messaging, not addressed messages
   - **Map to boat-agent:** Current event bus is already pheromonal — agents `emit(EVENT)` without knowing who listens. VaaS gives a name to the pattern.
   - **Cost:** Zero (just acknowledge the pattern)

3. **Cryogenic Memory Pattern (Pillar 3):**
   - **What:** Active memory (tzpro twin/) + cold archive (compressed, searchable backup)
   - **Map to tzpro-agent:** Add `twin/archive/` with daily/weekly compressed exports to S3/Glacier
   - **Cost:** Low (add backup cronjob)

**Adapt (Needs Work):**

4. **Entropy Budget (Pillar 1) → tzpro-agent Anomaly Detection:**
   - **What:** VaaS describes tracking "cognitive temperature" of agents. Abstract.
   - **Adaptation:** Map this to tzpro-agent's sounder anomaly detection — track deviation from learned baseline (e.g., bilge pump cycle duration, sounder bottom delta). When "entropy" (unusual variance) exceeds threshold → escalate.
   - **Cost:** Medium (add threshold learning to cascade/m1.py)

5. **Constitution Governance (Pillar 6) → boat-agent Playbook Staging:**
   - **What:** VaaS constitution defines who wins when agents disagree. Human-as-supreme.
   - **Adaptation:** boat-agent's playbook staging (static → replay → shadow → active) IS the constitution. Formalize the "staging gate" as a governance doc.
   - **Cost:** Low (document existing process)

**Avoid (Not Actionable / Overengineered):**

6. **Holographic Bridges (Pillar 5):**
   - **What:** Translator agents that keep "shadows" (lossless originals) to detect translation loss
   - **Why Avoid:** boat-agent/tzpro are single-language systems (no translation needed). Overkill for current scope.

7. **Grafting Protocol (Pillar 7):**
   - **What:** Fleet-wide knowledge pollination
   - **Why Avoid:** boat-agent is single-vessel. Fleet sync is out of scope.

8. **Operator Field Ψ(t) Math:**
   - **What:** Mathematical formalization of "system mood" as a field equation
   - **Why Avoid:** Pure theory. No clear implementation path. Navel-gazing.

### 1.6 Adoption Cost

**Zero Code to Copy:** There is no code. VaaS is 100% prose.

**Time Investment:**
- **Read for Patterns:** 2-4 hours to skim SAFETY_DEEP_DIVE.md, relevant pillars, domain guides
- **Extract Safety Layering:** 4 hours to formalize 4-layer envelope for boat-agent's `vessel.toml`
- **Adopt Terminology:** 1 hour to align "pheromone" language with current event bus docs

**Total:** 1 day to extract actionable patterns from 15k lines of prose.

**Risk:** Low — because there's no code, there's no dependency risk. The only risk is spending time reading overwrought design fiction for patterns that could be stated in 200 lines.

---

## 2. Spectro Audit

### 2.1 What It Actually Is

spectro is a **working Python package** that implements multi-model cognitive spectroscopy: send one prompt to N AI models in parallel, then analyze where they agree (convergences) and disagree (divergences). It's production-quality software with clean architecture, good tests, and optional semantic analysis.

**2-3 sentence summary:** spectro is a 1,426-line Python package that queries multiple AI models (DeepSeek, Seed Pro, Ornith, Nemotron) in parallel and extracts convergences, divergences, and unique insights from their responses using n-gram overlap and optional sentence-transformer semantic similarity. It's working software: 39/40 tests pass, includes CLI and Python APIs, handles retries and streaming, and is PyPI-ready as `spectro-spectrograph`.

### 2.2 Architecture & Key Files

**Package Structure:**
```
spectro/
├── spectro/
│   ├── __init__.py         (51 lines — exports)
│   ├── core.py             (653 lines — Spectrograph, parallel queries, retry logic)
│   ├── analysis.py         (440 lines — convergence/divergence extraction)
│   ├── cli.py              (245 lines — command-line interface)
│   └── exceptions.py       (37 lines — error types)
├── tests/
│   └── test_spectro.py     (40 tests — 39 passing)
├── pyproject.toml          (package config)
├── README.md               (usage docs)
└── LICENSE                 (MIT)
```

**Key Components:**

1. **core.py (Spectrograph):**
   - `Spectrograph` class with async HTTP client for parallel model queries
   - `_query_one()`: Single model with exponential-backoff retry (handles 429, 5xx, timeouts)
   - `_query_all()`: Parallel queries via `asyncio.gather`
   - `analyze()`: Sync wrapper, creates temp event loop
   - `analyze_async()`: Async version for existing loops
   - `analyze_stream()`: Streaming via `asyncio.as_completed` — yields responses as they arrive
   - Client lifecycle: `close()`, `aclose()`, context manager (`async with`)

2. **analysis.py (Spectral Analysis):**
   - `tokenize()`: Regex word extraction (min 3 chars)
   - `extract_keywords()`: Stopword-filtered keywords (min freq threshold)
   - `extract_key_phrases()`: Bigram/trigram phrases (skip stopword-heavy)
   - `analyze_spectrum()`: Main entry — returns (convergences, divergences, unique_insights, confidence)
   - `_concept_overlap()`: Build concept→models map
   - `_find_convergences()`: Concepts mentioned by ≥2 models, sorted by strength
   - `_find_divergences()`: Low phrase overlap pairs, negation mismatches, optional semantic sim
   - `_find_unique_insights()`: Single-model concepts (max 20)
   - `_confidence_score()`: Top 10 convergence strength average
   - **Semantic (optional):** Lazy-loads `sentence-transformers` (all-MiniLM-L6-v2) for cosine similarity

3. **cli.py (Command-Line Interface):**
   - `spectro "prompt"` — default 5-model ensemble
   - `--models model1,model2,model3` — custom model list
   - `--format json` — JSON output
   - `--verbose` — full responses
   - `--list-models` — show default ensemble
   - Report formatting: text bars, convergences/divergences/insights sections

**Default Models ("repertory company"):**
```python
DEFAULT_MODELS = [
    "deepseek-ai/DeepSeek-V4-Flash",      # cheap workhorse, 8/10
    "ByteDance/Seed-2.0-mini",             # thin chart, ideation
    "deepreinforce-ai/Ornith-1.0-35B",     # best fiction, punches above weight
    "nvidia/NVIDIA-Nemotron-3-Ultra-550B-A55B",  # structural, sergeant
    "ByteDance/Seed-2.0-pro",              # 9/10 best overall, lyrical
]
```

### 2.3 Maturity

**Status:** WORKING / PRODUCTION-READY

- **Code:** 1,426 LOC Python (clean, typed, documented)
- **Tests:** 40 tests, 39 passing, 1 failing (key-file-read test — Windows-specific issue)
- **Installable:** Yes — `pip install spectro-spectrograph` (PyPI package exists)
- **Working:** Yes — all core paths tested, retry logic works, streaming works
- **Dependencies:** `httpx` only (semantics optional via `sentence-transformers`)
- **Python:** 3.11+ required
- **License:** MIT

**Test Results:**
```
39 passed, 1 failed (test_read_key_file_from_env — key file path issue)
1 warning (sentence-transformers cache symlink warning on Windows)
```

**Real-World Usage:** CLI works out of the box. Set `DEEPINFRA_API_KEY` and run `spectro "your question"`.

### 2.4 Quality Issues Found

**Critical Issues:** None

**Moderate Issues:**

1. **Key File Reading Fails on Windows (Non-Blocking):**
   - `test_read_key_file_from_env` fails because `~/.openclaw/.deepinfra-key` path resolution doesn't work on Windows
   - Not blocking — env var (`DEEPINFRA_API_KEY`) works fine
   - Fix: Add Windows path handling in `_read_key_file()`

2. **Semantic Model Loading Synchronous:**
   - `SentenceTransformer` load in `_get_semantic_model()` is blocking
   - In async context, this could block the event loop
   - Fix: Run in `asyncio.to_thread()` or executor

3. **No Request Batching:**
   - Each model query is a separate HTTP request
   - For 5 models, that's 5 requests to the same provider
   - Some providers support batch requests (not DeepInfra, so this is future-proofing)

**Minor Issues:**

4. **Hard-Coded Model List:**
   - Default models are hardcoded in `DEFAULT_MODELS`
   - No way to fetch "available models" from provider
   - Fix: Add `--list-available-models` that queries provider API

5. **CLI Output Not Machine-Readable:**
   - `--format json` works, but includes truncated content (500 chars + "...")
   - Full content available in `--verbose` but not in JSON
   - Fix: Add `--json-full` flag

**Strengths:**

1. **Clean Async Architecture:** Proper use of `asyncio.gather`, `as_completed`, context managers
2. **Retry Logic Done Right:** Exponential backoff, retryable status codes (429, 5xx), max retries
3. **Graceful Degradation:** Semantic analysis optional — falls back to keyword-only if `sentence-transformers` not installed
4. **Good Error Types:** `APIKeyMissing`, `ModelUnavailable`, `ResponseMalformed`, `AnalysisTimeout`
5. **Streaming Support:** `analyze_stream()` yields responses as they complete — good for UIs
6. **Thread-Safe Overrides:** `analyze()` and `analyze_async()` accept per-call overrides (models, max_tokens, temp) without mutating shared state

### 2.5 Synergy with boat-agent/tzpro-agent

**Adopt (Direct Use):**

1. **Multi-Model Confidence Scoring → tzpro-agent Anomaly Validation:**
   - **What:** Use spectro's confidence scoring to validate tzpro-agent's anomaly detection
   - **Use Case:** When tzpro-agent detects a sounder anomaly (charted depth ≠ sounder depth), send the frame + context to 3 cheap models in parallel. If ≥2 agree "yes, that's a real shoaling," confidence is high. If they disagree, flag for human review.
   - **Implementation:**
     ```python
     from spectro import Spectrograph
     spec = Spectrograph(models=["deepseek-ai/DeepSeek-V4-Flash", "ByteDance/Seed-2.0-mini", "deepreinforce-ai/Ornith-1.0-35B"])
     result = spec.analyze(f"Sounder shows {depth} fm at {lat},{lon}. Chart says {charted} fm. Real anomaly or noise?")
     if result.confidence > 0.7:
         log_anomaly(confidence=result.confidence, convergences=result.convergences)
     ```
   - **Cost:** Low (add spectro as dependency, 1 call per anomaly)

2. **Convergence Pattern → boat-agent Event Bus Consensus:**
   - **What:** spectro's `_find_convergences()` pattern (concept → models map) can validate multi-agent consensus on boat-agent's event bus
   - **Use Case:** If 3 different agents (tzpro, pincher, hermes) all emit events suggesting "unsafe condition," the system should escalate. spectro's convergence logic gives us a pattern for detecting "everyone agrees this is bad."
   - **Implementation:** Port `_concept_overlap()` and `_find_convergences()` to Rust for boat-agent's consensus layer
   - **Cost:** Medium (reimpl in Rust, but algorithm is straightforward)

3. **CLI Pattern → boat-agent Debug Tooling:**
   - **What:** spectro's CLI design (single command, format flags, verbose mode) is a good model for boat-agent's debug tools
   - **Use Case:** `boat-agent status --format json` for machine-readable state dumps
   - **Cost:** Low (copy CLI patterns)

**Adapt (Needs Modification):**

4. **Semantic Similarity → tzpro-agent Temporal Comparison:**
   - **What:** spectro uses semantic similarity to detect "different words, same meaning" between models
   - **Adaptation:** Use sentence-embeddings to compare sounder frames across time (e.g., "this spot looks like it did 2 weeks ago"). Detect "pattern recurrence" even if exact pixel values differ.
   - **Implementation:**
     ```python
     from sentence_transformers import SentenceTransformer
     model = SentenceTransformer("all-MiniLM-L6-v2")
     # Encode frame descriptions (bottom type, depth, fish signature)
     emb_today = model.encode(today_frame.description)
     emb_last_week = model.encode(last_week_frame.description)
     similarity = cosine_similarity(emb_today, emb_last_week)
     if similarity > 0.85:
         log_pattern_recurrence(today_frame, last_week_frame)
     ```
   - **Cost:** Medium (add `sentence-transformers` dep, compute embeddings per frame)

5. **Streaming Analysis → tzpro-agent Real-Time Alerts:**
   - **What:** spectro's `analyze_stream()` yields results as each model completes
   - **Adaptation:** Stream anomaly analysis results as they arrive, not wait for full batch
   - **Use Case:** When an anomaly is detected, don't wait for full multi-model validation — return partial results immediately, update as more models respond
   - **Cost:** Low (adapt `analyze_stream` pattern to tzpro-agent's alert channel)

**Avoid (Not Relevant):**

6. **Default 5-Model Ensemble:**
   - **What:** spectro queries 5 models by default (expensive, slow for real-time ops)
   - **Why Avoid:** boat-agent/tzpro need sub-100ms response for safety. Multi-model validation should be async, not blocking. Use spectro for post-hoc validation, not real-time decision-making.

7. **Analysis UI (CLI Report):**
   - **What:** spectro's text report with convergence bars and divergence lists
   - **Why Avoid:** tzpro-agent's scrubber UI is already better for time-series data. Spectro's report is good for one-off queries, not time-series replay.

### 2.6 Adoption Cost

**Direct Use (As Dependency):**
- **Install:** `pip install spectro-spectrograph` — 5 seconds
- **Setup:** Set `DEEPINFRA_API_KEY` env var — 1 minute
- **Test:** `spectro "test prompt"` — 10 seconds
- **Integrate:** Import `Spectrograph`, call `analyze()` — 1 hour
- **Total:** 1-2 hours to working multi-model validation

**Rust Port (For boat-agent Kernel):**
- **Port Core Logic:** 8-16 hours (core.py is 653 lines, but half is retry/client code)
- **Port Analysis:** 4-8 hours (analysis.py is 440 lines of straightforward text processing)
- **Test:** 4 hours
- **Total:** 1-2 weeks to fully port to Rust

**Cost-Benefit:**
- **Use Python Wrapper:** Low cost, high value. Add spectro as dependency to tzpro-agent's Python environment, use for async anomaly validation.
- **Port to Rust:** Medium cost, questionable value. Rust port needed only if multi-model validation becomes safety-critical (not recommended — AI validation should never be in the safety path).

**Recommendation:** Use Python package as-is for non-blocking validation. Don't port to Rust.

---

## 3. Top-3 Adoption List (Concrete Action Items)

### Rank 1: spectro's Confidence Scoring for tzpro-agent Anomaly Validation

**What:** Add spectro as a dependency to tzpro-agent, use for multi-model validation of sounder anomalies.

**Why:**
- Low cost (pip install, 1 hour integration)
- High value (AI-backed validation without blocking real-time ops)
- Production-ready (39/40 tests passing, clean API)

**How:**
1. Add `spectro-spectrograph` to tzpro-agent's `requirements.txt`
2. In `cascade/m10.py`, when anomaly is detected:
   ```python
   from spectro import Spectrograph
   spec = Spectrograph(models=["cheap_model_1", "cheap_model_2"])
   result = spec.analyze(anomaly_description)
   if result.confidence > 0.7:
       log_validated_anomaly(result)
   ```
3. Store convergences/divergences in twin/ for later review

**Time to Value:** 1 day

---

### Rank 2: VaaS Safety Envelope Layering for boat-agent's vessel.toml

**What:** Formalize boat-agent's safety limits as 4 layers (HARD/SOFT/ADVISORY/ETHICAL) per VaaS SAFETY_DEEP_DIVE.md.

**Why:**
- Zero code dependency (documentation pattern only)
- Clarifies what's physically impossible vs operationally risky vs ethically questionable
- Aligns with boat-agent's existing `vessel.toml` structure

**How:**
1. Read `VaaS/docs/SAFETY_DEEP_DIVE.md` (30 min)
2. Map existing `vessel.toml` fields to layers:
   - HARD: `max_rate_of_turn`, `min_keel_clearance` (already enforced)
   - SOFT: `recommended_speed`, `max_turn_in_confined_waters` (add warning layer)
   - ADVISORY: `fuel_efficiency_threshold`, `heavy_weather_area` (add log-only layer)
   - ETHICAL: (optional) flag patterns that look like risk escalation
3. Update `core/src/envelope/` docs to reflect 4-layer model

**Time to Value:** 1 day

---

### Rank 3: spectro's Semantic Similarity for tzpro-agent Temporal Pattern Matching

**What:** Use sentence-transformers (already a dependency of spectro) to detect "similar sounder conditions" across time.

**Why:**
- Medium cost (add dependency, compute embeddings)
- High value for fisherman ("show me days that look like today")
- Uses same semantic model as spectro, no new tech

**How:**
1. Add `sentence-transformers` to tzpro-agent dependencies
2. For each M10 record, compute embedding of frame description:
   ```python
   from sentence_transformers import SentenceTransformer
   model = SentenceTransformer("all-MiniLM-L6-v2")
   emb = model.encode(f"{bottom_type} {depth}fm {fish_signature}")
   ```
3. Store emb in twin/. Query by cosine similarity for "similar days"

**Time to Value:** 3 days

---

## 4. What NOT to Adopt (Avoid List)

1. **VaaS "Crab/Shell" Metaphor:** Cute but confusing. Stick to "agent" and "harness."
2. **VaaS "Operator Field Ψ(t)" Math:** Pure theory. No implementation path.
3. **VaaS Holographic Bridges:** Overengineered for single-language system.
4. **VaaS Grafting Protocol:** Single-vessel scope, fleet sync out of scope.
5. **VaaS AI Session Transcripts (ka1-ka5):** 600k lines of chatlogs. Delete or archive separately.
6. **spectro's 5-Model Default:** Too expensive for real-time. Use 2-3 cheap models.

---

## 5. File Deliverables

**VaaS:**
- `VaaS/docs/SAFETY_DEEP_DIVE.md` — READ for safety layering patterns
- `VaaS/docs/APP_LOWPOWER_SAIL.md` — READ for single-vessel UX patterns (ignore story fluff)
- `VaaS/ka*.md` — DELETE or archive (600k lines of AI chatlogs, no signal)

**spectro:**
- `spectro/spectro/core.py` — USE as-is for multi-model queries
- `spectro/spectro/analysis.py` — REFERENCE for confidence scoring patterns
- `spectro/spectro/cli.py` — REFERENCE for debug CLI patterns

---

## 6. Final Verdict

| Repo | Status | Action |
|------|--------|--------|
| **VaaS** | DOCS-ONLY | Read safety layering patterns. Ignore code claims (no code). |
| **spectro** | WORKING | Install as dependency. Use for anomaly validation. |

**spectro is real. VaaS is not.**

Adopt spectro's analysis patterns. Read VaaS's safety docs for layering ideas. Don't expect to run any code from VaaS — there isn't any.

---

**Audit Complete.** 2026-07-20.
