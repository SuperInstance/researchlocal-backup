# Beta Diary — hermes-construct

## Who I Am

I'm Riley. Senior Rust dev at a mid-size startup. I build CLI tools and backend services — the kind of thing that ships in a Docker container and runs for months without hand-holding. I came to hermes-construct looking for an AI agent framework I could embed in our Rust services. I'm skeptical but curious. I clone the repo and open it like I'd evaluate any dependency we're considering.

---

## First Impression (score: 3/10)

The README is *gorgeous*. Clear value prop in the first paragraph. Beautiful feature table. Explains what "construct" means. Links to module repos. The whole thing reads like a product page.

Which is exactly the problem.

I read the whole README and I still can't tell you what `cargo add hermes-construct` gives me. There's no code example showing the API surface. There's no "Quick Start for Developers" section. The install instructions tell me to run a curl script and then clone the fork — that's an end-user install, not a developer integration.

The README talks about `cargo add crackle-runtime` for individual modules, which is good — those module repos exist and have actual Rust code. But hermes-construct itself? It's a binary crate. No `lib.rs`. I can't `use hermes_construct::anything` in my own project.

The first 10 seconds tell me this is an *agent* (a chatbot with Telegram integration), not a *framework* (something I build on). The README doesn't make that distinction clear.

---

## Code Quality (score: 6/10)

### The Good

The Rust code that exists is solid. Well-structured, consistent style, good use of types. Let me break it down:

- **126 tests, all passing.** This isn't trivial. The tests cover real behavior: SQLite CRUD, conservation budget tracking, room routing, tile lifecycle, ensign state machines, correlation detection, module load/unload, and the full kernel message pipeline.
- **No `unsafe` anywhere.** Zero instances across 6,400+ lines of Rust. Good.
- **Error handling is correct.** Uses `thiserror` for error types, `Result<T, String>` in places where custom errors would be overkill. No `unwrap()` in production paths (only in tests).
- **SQLite usage is competent.** WAL mode, proper schema initialization, upserts, prepared statements, busy timeouts. Whoever wrote this knows their SQLite.
- **The module system is well-designed.** Clean `Module` trait with load/unload lifecycle, `ModuleRegistry` with proper state tracking, `AutoLoader` with keyword matching. The trait is `Send + Sync`, which is correct for async runtimes.

### The Bad

- **`cargo clippy -- -D warnings` fails with 38 errors.** All dead code warnings. This means the crate defines a lot of public API surface that nothing actually calls. It's scaffold, not implementation. `TILE_COMPLETE`, `TILE_ARCHIVE`, `ENSIGN_ORIENT`, `ENSIGN_STAND_DOWN`, `GRAVITY_RECALIBRATE`, `PHONE_A_FRIEND`, `CORRELATION_TRANSFER`, `PENROSE_REFIT`, `PORT_OPEN_CLOSE`, `PORT_MESSAGE`, `DEADBAND_ACTION`, `BOOTSTRAP_STEP`, `SHELL_SPAWN`, `SHELL_DESTROY` — these are all defined conservation cost constants that nothing uses yet. The deadband module has a full `DeadbandState` struct with `new()`, `is_in_band()`, `update()`, and `detect_trend()` — none of which are called outside tests. It's infrastructure waiting for a purpose.

- **No `lib.rs`.** This is a `[[bin]]` crate. You cannot use this as a library. The `Cargo.toml` has no `[lib]` section. If you want to embed hermes-construct in your own Rust service, you'd have to fork it and add one yourself. For something that claims to be a "construct you assemble from parts," this is ironic.

- **Examples duplicate all types inline.** Look at `examples/basic_agent.rs` — it literally re-declares every struct, trait, and function inside a `mod hermes {}` block because there's no library to import from. That's 400+ lines of duplication in a single example. If you change the real `kernel.rs`, the example breaks silently because it's using its own copy.

- **Room routing is keyword matching.** `route_to_room()` checks if the message contains "navigate", "build", "debug", etc. This is documented as "simple" and "would use NLP/embedding similarity" in a full implementation — but there's no indication that NLP routing is actually planned or in progress. For a project that talks about JEPA gravity and spectral topology analysis, keyword matching for the core routing function is a yawning gap between promise and delivery.

- **Provider implementations are stubs.** `DeepInfraProvider` and `ZaiProvider` in `ensign.rs` are defined but their `complete()` methods construct a real HTTP request structure — but there's no indication they actually hit the API. The `MockProvider` in tests returns `format!("echo: {}", req.prompt)`. The real providers might work — I can't test without API keys — but the structure suggests they're scaffold.

---

## Architecture (score: 5/10)

The abstractions are interesting but unevenly implemented.

### Room-Native Architecture
The room concept is solid. Each room has its own gravity, temperature, max_tokens, prompt_style, and conservation budget. Rooms are loaded from JSON configs, stored in SQLite, and their gravity evolves over time (decay toward zero, nudged by interaction signals). This is a genuinely novel approach to context management. The gravity-to-params mapping (`gravity.rs`) derives model parameters from a single f64, which is elegant even if the mapping is linear and simple.

But: rooms don't actually do anything the upstream Hermes Agent doesn't already do differently. The upstream has "sessions" with model configs. This has "rooms" with gravity-derived configs. The room system adds complexity without clear benefit over a simpler approach like "let the user pick a temperature."

### Ensign Agents
The ensign lifecycle (Dormant → Waking → Oriented → YellowAlert → Active → StandingDown) is a proper state machine. The escalation model (cheap model watches, expensive model handles problems) is a real architectural pattern. But the implementation is incomplete — ensigns don't actually run background checks. The lifecycle exists, the SQLite persistence exists, but there's no event loop that fires `ensign.check()` on a timer. The ensigns are wired into the message pipeline (activated when a message comes in, charged for the activation) but they don't independently monitor anything.

### Module System
The best-designed part of the codebase. The `Module` trait, `ModuleRegistry`, and `AutoLoader` form a clean, usable module loading system. The keyword-based matching is primitive but functional. The load history tracking is a nice touch. This is the part I'd actually want to use.

Except I can't. Because there's no `lib.rs`.

### Onboarding Wizard
It's a CLI wizard that asks your name, role, and preferred channel. It generates a TOML config. It's fine. It's not charming, not cringe — it's functional. The role presets (Developer, Researcher, Writer, Sysadmin, DataAnalyst) are reasonable. The wizard has 618 lines of code, which seems like a lot for what it does — most of that is display formatting and input validation. It's well-tested though.

### Conservation Tracking
Every operation has an energy cost. The conservation state is persisted in SQLite. When budget is low, the kernel degrades gracefully (throttled mode → floor mode). This is a genuinely useful feature for production deployments where you need to cap API spending. The three-tier degradation (Full → Throttled → Floor) is well-designed.

But the costs are all arbitrary constants with no basis in reality. `TILE_CREATE = 0.1`, `ENSIGN_ACTIVATE = 1.0`, `GRAVITY_UPDATE = 0.05`. What unit is this? Tokens? Dollars? Joules? The README says "energy" but the constants don't correspond to any real metric. You'd need to calibrate these for your actual API costs, and there's no tooling to help with that.

### Provenance
Every decision gets a provenance entry: what model was used, what provider, what params, what conservation cost, what decision was made (normal/throttled/floor-refusal), linked to parent tiles. This is genuinely valuable for debugging and auditing. It's the kind of thing that would make a compliance team happy.

---

## What's Missing

- **No `lib.rs`.** Can't use this as a library. Period. The biggest dealbreaker.
- **No public API documentation.** No rustdocs. No `cargo doc` output worth reading.
- **No integration examples.** The examples duplicate types inline instead of importing from the crate. They don't demonstrate real-world usage patterns.
- **No actual module loading.** The module system uses `StubModule` everywhere. The referenced modules (crackle-runtime, conservation-checker, etc.) exist as separate repos but there's no code that dynamically loads them. The `main.rs` registers stubs.
- **No real ensign monitoring.** Ensigns have a lifecycle but don't run background health checks.
- **No error recovery.** If the kernel hits an error, it logs and continues. There's no retry logic, circuit breaker, or backpressure.
- **No configuration validation.** Room JSON files are parsed with serde defaults, but there's no validation that gravity is in [-1, 1], that conservation budgets are positive, etc.
- **No graceful degradation for missing providers.** If no provider is configured, the kernel will log a warning but still try to route messages to rooms that need providers. It'll error at runtime.
- **No metrics or observability.** No Prometheus endpoint, no structured logging (just `log::info!`), no tracing spans.
- **The Python/Rust split is confusing.** The repo contains a full Python codebase (upstream Hermes Agent, 32k+ lines) AND a Rust binary. They share no code. The README doesn't clearly explain the relationship. Are you supposed to run both? Is the Rust binary replacing the Python CLI? Is it a companion?

---

## What's Good

- **The module system trait design.** Clean, composable, testable. The `Module`, `ModuleRegistry`, and `AutoLoader` are well-factored. If there were a `lib.rs`, I'd `cargo add` this just for the module system.
- **Conservation budget with three-tier degradation.** Real production concern, well-implemented. The Floor mode that refuses work honestly instead of silently failing is good engineering.
- **Provenance tracking.** Every decision is recorded with full context. This is audit-grade logging.
- **126 passing tests.** Not just compilation tests — real behavior tests. SQLite integration tests. State machine lifecycle tests. Kernel pipeline tests. This is better test coverage than most crates I've evaluated.
- **Room gravity is a genuinely novel idea.** Deriving model parameters from a single scalar that evolves over time is interesting. It's not fine-tuning, but it's a cheap approximation that could work in practice.
- **SQLite WAL mode done right.** Busy timeout, checkpoint on shutdown, proper upserts. Whoever wrote the persistence layer knows their trade.
- **Zero `unsafe`.** Always nice to see in a systems project.
- **The separate module repos are real.** crackle-runtime, conservation-checker, negative-space-testing, cathedral-probe, spacemap — these all exist on GitHub with actual Rust source code. They're not vaporware.
- **Room JSON configs are well-structured.** The configs have sensible defaults, escalation models, conservation budgets, and allowed modules per room. Good separation of config from code.
- **The ensign alert levels make sense.** Yellow alert = actively monitoring, even when things are fine. This matches real-world operations.

---

## Dealbreakers

1. **No `lib.rs`.** I cannot embed this in my project. I can't `cargo add hermes-construct`. I'd have to copy-paste source files. That's not a dependency, that's a fork.
2. **The Python elephant.** This repo contains a 32,000-line Python codebase (upstream Hermes Agent) that does everything the Rust code aspires to do, plus 100x more. The Rust code is 6,400 lines. The Python code has 15+ messaging platforms, 6 terminal backends, a web dashboard, cron scheduling, a plugin system, voice support, and image analysis. The Rust binary has Telegram and stdio. Why would I use the Rust version?
3. **Clippy fails.** 38 dead code warnings. This means the API surface is aspirational, not implemented. The cost constants, deadband state, ensign methods, tile queries — they're defined but unused. It compiles, but it's not done.
4. **Single commit history.** The entire Rust codebase was added in one commit. No evolution, no code review trail, no iterative development. This makes it hard to trust the code's maturity.

---

## Would I Use This?

**No. Not today.**

The ideas are interesting. The code quality is higher than I expected. The module system design is genuinely good. But I can't use it because:

1. It's not a library.
2. The Python upstream is 50x more mature.
3. The Rust code is a prototype, not a product.

If the module system were extracted into its own crate — `hermes-modules` or whatever — with a proper `lib.rs`, rustdocs, and integration examples, I'd consider it. The conservation tracking and provenance logging would be useful standalone too.

As it stands, hermes-construct is a promising prototype of what the Rust layer *could* be, bolted onto a mature Python agent that already does everything. The Rust code doesn't add enough value over the Python base to justify the complexity of running both.

**Would I watch it?** Yes. If they extract the Rust parts into proper library crates, add a real module loading mechanism (dynamic loading, not stubs), and make the room/gravity system available as an embeddable library, I'd revisit.

---

## Specific Recommendations

1. **Add `lib.rs` immediately.** Re-export the public types. Make `Module`, `ModuleRegistry`, `AutoLoader`, `Room`, `Tile`, `ConservationState`, and `ShellKernel` available as a library. This is a one-afternoon change.

2. **Fix the 38 clippy warnings.** Either use the dead code or remove it. Don't ship scaffold that looks like API.

3. **Write rustdocs.** Every public type, trait, and function should have a doc comment with an example. The module system especially — developers need to see how to implement a custom module.

4. **Fix the examples.** They should `use hermes_construct::*;`, not duplicate 400 lines of types inline. Examples that import from the crate prove the crate is usable as a dependency.

5. **Add a `CHANGELOG.md`.** The repo has 15 `RELEASE_*.md` files for the Python upstream but nothing for the Rust code. I need to know what changed between versions.

6. **Clarify the Python/Rust relationship in the README.** Is the Rust binary a replacement for the Python CLI? A companion? A future direction? Right now it's confusing.

7. **Calibrate conservation costs.** The arbitrary constants need to be grounded in something. Either document that they're abstract units and the user should calibrate, or provide a calibration tool.

8. **Implement real module loading.** The stubs are fine for a demo, but the `AutoLoader` should actually load compiled modules. Either use `libloading` for dynamic loading or document that modules are compile-time dependencies.

9. **Add room routing tests for all room types.** Currently only Navigation has a routing test. Engineering, Science, Security, and Social should all have tests.

10. **Split the repo.** The Rust code and the Python code don't share anything. Put the Rust code in its own repo. It's confusing to have both in one repo, especially when the Python code is 5x the size.

---

*Diary written by Riley, senior Rust dev, on 2026-06-01.*
*hermes-construct v0.1.0, commit f32448f.*
*Evaluation took about 90 minutes.*
