# 🧪 Hermes Construct — Real Test Diary

**Date:** 2026-06-01  
**Tester:** Dmitri (agent framework developer)  
**Crate:** `hermes-construct` v0.1.0 on crates.io  
**Source:** github.com/SuperInstance/hermes-construct  
**Commit (HEAD):** shallow clone, depth 1

---

## 1. First Impression — crates.io & README

**crates.io page:** Exists. Name is OK. Description says: *"tile-operating shell kernel for Oracle ARM"*. This immediately feels off — Oracle ARM? What does ARM have to do with an agent framework?

**README:** The README is... a lot. It opens with a massive ASCII art banner, describes "ceremonial" rooms (Xibalba, PLATO, Cathedral), talks about "Crackle" (a JEPA-like predictive coding module), "Spacemap" (provenance in latent space), and a conserved budget that models LLM costs as a "thermodynamic fuel". There's a huge dependency tree diagram. Half of it reads like a manifesto, half like actual architecture docs.

**Verdict:** The README has genuine technical ideas mixed with heavy roleplaying. It's trying too hard to be cool. The Oracle ARM reference seems either wrong or refers to something niche — I searched and couldn't connect it to this project. **Feels like vaporware energy at first glance, but digging deeper reveals actual engineering.**

**Score so far:** ⭐⭐ (let's see if the code redeems it)

---

## 2. Build — cargo test & clippy

```bash
$ cargo test
test result: ok. 209 passed; 0 failed; 0 ignored
$ cargo clippy
# minor warnings: unused variable in test, unused mut, dead_code in example
```

**209 tests. Zero failures.** That is genuinely impressive for a v0.1.0. Most early crates have 30-60 tests covering basic smoke tests. This has full coverage across:

- **Conservation (cost model):** 16 tests — pricing for GPT-4o, Claude Opus, GLM Flash, cost blends, budget tracking, persistence
- **Ensign:** 10 tests — lifecycle (wake, orient, go red, stand down), message formatting, SQLite upsert
- **Gravity:** 16 tests — JEPA gravity→params mapping, sigmoid, temperature, top_p, frequency penalty, prompt style interpolation
- **Room:** 8 tests — routing (keyword match), gravity decay/nudge, model params, SQLite
- **Kernel:** 8 tests — full message processing pipeline, conservation spending, metrics
- **Spectral:** 44 tests (!) — operator algebra (trace, norm, compose), Dirac spectrum, Wasserstein distance, renormalization group flow, variational optimization, Berry phase
- **Penrose:** 18 tests — Pearson correlation, autocorrelation, cross-correlation, peak detection, classification
- **Deadband:** 14 tests — band checking, trend detection, zero-setpoint edge case, consecutive breaches
- **Module:** 15 tests — load/unload/register, capability matching, cost estimation, autoload
- **Tile:** 10 tests — CRUD, builder pattern, escalation
- **Port:** 9 tests — stdio/telegram ports, LIFO, active flag
- **Onboarding:** 18 tests — wizard flow, role presets, config generation, confirmation variants
- **Additional:** renormalization group fixed points, variational optimization convergence, etc.

**What these tests cover:**
- Unit tests for every module
- SQLite persistence (in-memory integration)
- Edge cases (zero inputs, empty series, clamped values)
- The spectral module has genuine numerical stability testing

**What they DON'T test:**
- No integration test that runs the full binary end-to-end
- No network-dependent tests (Telegram, model APIs)
- The `basic_agent` example is the closest thing to an integration test

**Verdict:** The testing is **exceptionally thorough for a v0.1.0**. The author cares about correctness. This is not a weekend hack.

---

## 3. Architecture — Module-by-Module Deep Read

### Overall Structure

```
src/
├── main.rs         # bootstrap + tick loop
├── lib.rs          # module exports
├── kernel.rs       # ShellKernel — main tick loop, message routing, provider dispatch
├── room.rs         # Room types, gravity state, keyword routing
├── ensign.rs       # Provider abstraction, lifecycle, call tracking
├── gravity.rs      # JEPA gravity → model parameter mapping
├── conservation.rs # Budget tracking, cost table, SQLite persistence
├── module.rs       # Runtime module system with capability registry
├── spectral.rs     # 🚩 Spectral triple, Wasserstein, RG flow, Berry phase
├── penrose.rs      # Cross-room Pearson correlation, autocorrelation
├── deadband.rs     # Deadband monitoring + trend detection
├── tile.rs         # Tile types + SQLite CRUD
├── port.rs         # Port trait + Telegram adapter
└── onboarding.rs   # First-run wizard
```

### The Good

**Kernel architecture is solid.** The tick loop is simple (receive → route → process → tick), and the provider abstraction through `Ensign` + `Provider` trait is clean. The conservation budget as a central resource model is a genuinely interesting design constraint — every action has a cost, and the system degrades gracefully when budget runs low.

**Gravity → model params mapping is well-implemented.** The JEPA (Joint Embedding Predictive Architecture) framing is a fancy name for what's essentially a sigmoid-based parameter interpolation system, but it's well-designed: gravity as a value in [-1, 1] that maps to temperature, max_tokens, prompt_style, top_p, and frequency_penalty. Clean, testable, documented.

**SQLite schema is sane.** Proper foreign keys, indexes, parameterized queries. The code uses `rusqlite` with bundled SQLite. No ORM overhead.

**The module system** is clean and straightforward. Capability registry with task routing. Simple but effective.

**Deadband monitoring** is a genuinely useful operational pattern. Real-time value monitoring with trend detection (stable/drifting/oscillating/diverging). This is the kind of thing you'd actually want in a production agent system.

**Penrose correlation detection** — Pearson correlation, autocorrelation, and cross-correlation are well-implemented. The spline type classification is a nice way to categorize inter-room relationships. This is useful operational intelligence.

### The Concern

**Spectral.rs.** Oh boy. This module is 900+ lines of noncommutative geometry in an agent framework. It contains:

- `SpectralTriple` (Connes' (A, H, D) from noncommutative geometry)
- `BoundedOperator` with Jacobi eigenvalue decomposition
- `DiracOperator` with spectrum computation
- Wasserstein distance (Sinkhorn algorithm)
- Renormalization group flow (beta function, fixed points)
- Variational principle for room optimization
- Berry phase computation

This is **deeply over-engineered** for an agent framework. It has 44 tests and they all pass. The math is real. But it's completely disconnected from the rest of the codebase — nothing in `kernel.rs`, `room.rs`, or any example actually uses the spectral triple for anything. It's a library of advanced mathematics that someone carefully implemented and then... parked.

**Is it used?** I searched all source files for references to `SpectralTriple`, `BoundedOperator`, `DiracOperator`, `wasserstein`, `RenormalizationFlow`, `berry_phase`, `variational_optimize`. Zero usage outside spectral.rs and its tests. It's **pure suspension bridge** — beautiful, complex, serves no practical function.

### The Weird

**PLATO / Xibalba / Cathedral rooms.** The room system treats rooms as "ceremonial spaces" with specific types like `Engineering`, `Navigation`, `Social`, `Plato`, `Xibalba`, `Cathedral`. The Oracle ARM reference in the README/cargo description appears nowhere in the code. The onboarding wizard suggests role-based rooms ("engineering", "science", "creative", "monitoring") — these are useful. The ceremonial names are just wordplay.

**The crate description says "Oracle ARM"** but the code is a generic agent framework with no ARM-specific code. This is confusing and likely either a mistake, an inside joke, or a reference to a fork target that never materialized.

---

## 4. Real Test — Building a Simple Agent

I ran `cargo run --example basic_agent` and it worked on the first try. The example:

1. Creates SQLite tables
2. Sets up 3 rooms (engineering, navigation, social)
3. Deploys ensigns with mock provider
4. Routes messages by keyword matching
5. Calls mock provider
6. Records tiles
7. Tracks conservation budget

The full flow is only ~100 lines of Rust. The ergonomics are reasonable for Rust — the `Provider` trait is `#[async_trait]`, SQLite operations are explicit, and the kernel handles routing.

**Could an outsider do this?** Yes, if they know Rust. The `basic_agent` example is literally copy-pasteable. The `MockProvider` pattern means you can test without API keys. The onboarding wizard even generates TOML configs.

**For a real agent with actual LLM calls**, you'd need:
1. Replace `MockProvider` with an actual `Provider` implementation (e.g., `OpenAIProvider` or `DeepInfraProvider`)
2. Set up env vars for API keys
3. Optionally wire up the Telegram port

The infrastructure is there but the actual providers aren't. The `Provider` trait expects you to implement them.

---

## 5. Is This Useful for Building Real Agent Systems?

**Short answer:** Yes, but with significant caveats.

### What's useful:

1. **Cost-aware execution model** — The conservation budget is a genuinely good idea. Every action has a cost, and the system knows its limits. This prevents runaway API costs.

2. **Room-based routing** — Keyword-based message routing to specialized rooms is simple and effective. The gravity system gives each room a persona.

3. **Deadband monitoring** — Real operational intelligence. If you're running a multi-agent system, deadband alerts when things drift are invaluable.

4. **Penrose correlation** — Cross-room correlation detection is useful for understanding system behavior. "Why did engineering start producing worse responses when social was overloaded?"

5. **Module system** — Simple capability registration is better than monolithic agents.

### What's NOT useful:

1. **Spectral triple / noncommutative geometry** — Beautiful but completely unused. If you have a PhD in operator algebras, you might enjoy it. Otherwise, it's dead weight.

2. **Renormalization group flow** — Budget evolution modeled as RG flow is intellectually elegant but practically meaningless. You can track your budget with a simple counter.

3. **Berry phase** — In an agent framework. I cannot overstate how unnecessary this is.

4. **No provider implementations** — The crate provides the `Provider` trait but no actual HTTP-based providers. You have to write your own OpenAI/Anthropic/DeepInfra clients.

5. **The naming** — "Plato", "Xibalba", "Cathedral", "Ensign", "ZeroClaw" — this isn't a framework, it's a worldbuilding exercise. It makes the code harder to navigate and looks unprofessional.

6. **Oracle ARM** — Whatever this is, it doesn't match the code. This alone would make me hesitate to use the crate.

### What's missing for production:

- **No auth** — How do rooms authenticate with different providers?
- **No retry logic** — Provider calls fail? Unhandled.
- **No rate limiting** — Conservation budget helps, but no API rate limit management.
- **No observability beyond SQLite** — Metrics are an in-memory snapshot. No tracing, no logging levels.
- **No built-in state machines** — Tiles have statuses but no transition logic.
- **No agent-to-agent communication protocol** — Rooms are isolated SQLite spaces.
- **No streaming** — Provider trait returns CompletedResponse, not streaming.

---

## 6. Final Score

| Category | Score | Notes |
|---|---|---|
| **Code quality** | ⭐⭐⭐⭐ | Clean Rust, good tests (209), no unsafe, well-factored modules |
| **Test coverage** | ⭐⭐⭐⭐⭐ | 209 tests for a v0.1.0. Exceptional. |
| **Documentation** | ⭐⭐ | README is long but confusing. Module docstrings are good. No API docs. |
| **Architecture** | ⭐⭐⭐ | Good core (kernel/ensign/conservation). Spectral module is bloat. |
| **Practical usefulness** | ⭐⭐ | Good ideas, but no provider implementations, no streaming, no production tooling. |
| **Honesty (crates.io + README)** | ⭐ | Oracle ARM is misleading. README is part manifesto, part docs. |

## Overall: ⭐⭐½ (2.5 / 5)

**Breakdown:**

- **If you want to build an agent system today:** Use `rig`, `llm-chain`, or just call APIs directly. Hermes Construct has interesting ideas but isn't ready.

- **If you're researching agent architectures:** Steal the conservation budget model and the gravity→params mapping. The deadband and penrose modules are genuinely good operational patterns.

- **If you like weird Rust projects:** This is fun. The spectral module is a beautiful distraction. Run the examples, enjoy the ambiance, but don't deploy it.

- **The honest take:** This is someone's passion project with real engineering talent behind it. The core architecture (kernel + ensign + conservation + gravity) is solid and well-implemented. But the spectral module represents ~30% of the codebase and does nothing. The naming is impenetrable. The crate description is actively misleading. If the author stripped out the non-essential math, provided real provider implementations, and clarified the naming, this could be a ⭐⭐⭐⭐ crate. Today it's a ⭐⭐½ that I'll keep an eye on.

---

## Appendix: Files Read

- `README.md` (full)
- `Cargo.toml`
- `src/lib.rs`, `src/main.rs`
- `src/room.rs`, `src/ensign.rs`, `src/kernel.rs`
- `src/gravity.rs`, `src/conservation.rs`
- `src/module.rs`, `src/tile.rs`
- `src/spectral.rs` (full — 900+ lines)
- `src/penrose.rs` (full)
- `src/deadband.rs` (full)
- `src/port.rs`, `src/onboarding.rs`
- `examples/basic_agent.rs`, `circuit_demo.rs`, `correlation_demo.rs`, `sandbox_demo.rs`, `provenance_demo.rs`

**Tests run:** `cargo test` — 209 passed, 0 failed, 0 ignored.
**CLI check:** `cargo clippy` — minor warnings, no errors.
**Examples run:** `basic_agent` — works on first try.
