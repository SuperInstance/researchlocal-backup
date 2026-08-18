# FLUX/PLATO Synergy Analysis for boat-agent

**Date:** 2026-07-19
**Scope:** Analysis of FLUX/conservation and PLATO room repos for integration with boat-agent OS
**Analyst:** Claude (Opus 4.5)

---

## Executive Summary

The FLUX/PLATO ecosystem offers three **high-value, low-cost adoption opportunities** for boat-agent:

1. **conservation-enforcer-rs** — Deterministic policy bytecode that could replace our playbook stage-gate Python validation with provably auditable enforcement
2. **plato-room-deployment-approval** — Human approval gate pattern directly applicable to our playbook APPROVED stage
3. **óthismos** — Pressure measurement framework for detecting when our safety envelope is being "pushed against"

However, **critical gaps** exist: FLUX bytecode is designed for text/LLM governance, not real-time control loops. The envelope would need a realtime policy dialect.

---

## Cluster Overview

| Repo | Maturity | What It Actually Is |
|------|----------|---------------------|
| **flux-runtime** | Working (2037 tests) | Python FLUX bytecode VM with markdown→bytecode compiler |
| **flux-core** | Working (51 tests) | Rust FLUX VM, register-based, A2A protocol |
| **conservation-enforcer** | Working (60+ tests) | Python LLM output governor via FLUX bytecode |
| **conservation-enforcer-rs** | Working (150+ tests) | Rust conservation enforcer, zero-dep, WASM-ready |
| **flux-policy-tester** | Working | Policy test harness (unit, fuzz, bounds) |
| **snapkit-flux-bridge** | Prototype | Translates snapkit Governor configs to FLUX bytecode |
| **othismos** | Working (135 tests) | Pressure measurement framework (`Π = ‖Δθ‖`) |
| **othismos-llm** | Working | Context truncation pressure for LLMs |
| **plato-core** | Working | Python PLATO protocol (tiles, mesh registry) |
| **plato-core-rs** | Working (40+ tests) | Rust PLATO room implementation |
| **plato-portal** | Docs-only | SuperInstance SDK (aspirational roadmap) |
| **plato-room-code-review** | Working | Automated PR review as PLATO room |
| **plato-room-security-audit** | Working | Security audit room (heuristic checks) |
| **plato-room-deployment-approval** | Working | Deployment gating room with conservation laws |

---

## Detailed Analysis

### FLUX Runtime Family

#### 1. flux-runtime (Python)

**What it is:** Markdown-to-bytecode VM with 2037 tests. Compiles `.ese` files (structured markdown) to FLUX bytecode and runs on a 64-register VM.

**Maturity:** Working, production-ready, zero dependencies

**Concrete synergy:**
- Could provide **playbook verification** — compile playbooks to bytecode to prove determinism before deployment
- Vocabulary system could express maritime control patterns as natural language that compiles to envelope checks

**Adoption cost:** Adapter — would need to define maritime-specific vocabulary and prove bytecode maps 1:1 with Python playbook logic

**Verdict:** Nice-to-have for playbook authoring, but not critical for safety path

---

#### 2. flux-core (Rust)

**What it is:** Register-based FLUX VM in Rust. Same bytecode ISA as Python version. 16 GP registers, 16 FP registers, cycle budgets, A2A protocol for multi-agent messaging.

**Maturity:** Working, 51 tests, published on crates.io as `fluxvm`

**Concrete synergy:**
- Could run **inside our kernel** as a deterministic policy evaluation layer
- A2A protocol (TELL, ASK, DELEGATE, BROADCAST) could enable multi-agent coordination on the vessel
- Swarm consensus (majority voting) could be used for redundant sensor fusion

**Adoption cost:** Drop-in for computation, but **no maritime semantics** — would need to define what "consensus" means for rudder position

**Verdict:** Useful for future multi-agent work, not immediate value

---

### Conservation Enforcement Family

#### 3. conservation-enforcer (Python)

**What it is:** FLUX bytecode governor that wraps LLM calls. Enforces seven conservation laws: length budget, repetition limit, category confinement, entropy floor, information density, scope discipline, budget decay.

**Maturity:** Working, 60+ tests

**Concrete synergy:**
- **Pattern match for our playbook stage-gate** — conservation-enforcer is exactly what we're trying to build, but for text output vs. actuator commands
- Our ENVELOPE layer is already a conservation law (rate limits, bounds)

**Adoption cost:** Research project — the conservation laws are text-specific (token count, word overlap), not physical

**Verdict:** Study the architecture, but can't reuse code directly

---

#### 4. conservation-enforcer-rs ⭐

**What it is:** Rust conservation enforcer with FLUX VM, assembler, enforcement layer, policies, audit logging. Zero external dependencies, WASM-ready, `no_std` capable.

**Maturity:** Working, 150+ tests, production-ready

**Concrete synergy:**
- **The most directly applicable component** — could define physical conservation laws (rudder rate, throttle rate, heading oscillation) as FLUX bytecode
- Zero-dep Rust VM could run **inside our envelope** as the final arbiter
- Audit logging (feature-gated) already matches our black box requirements
- Cycle budget enforcement prevents runaway policies

**Adoption cost:** Adapter — need to define physical conservation laws and write policy bytecode, but the VM is ready

**Key insight:** Our envelope already implements these checks in pure Rust. The value proposition is **bytecode as audit trail** — a compiled `.bin` policy file that can be inspected, versioned, and proven to match vessel.toml.

**Verdict:** **HIGH VALUE** — adopt for policy auditability and staged rollout of new envelope rules

---

#### 5. flux-policy-tester

**What it is:** Testing framework for FLUX policies — unit tests, adversarial fuzzing, property-based testing, conservation bound verification.

**Maturity:** Working

**Concrete synergy:**
- Could be used to **test envelope policies** against edge cases (extreme sensor values, NaN, rate limits)
- Fuzz testing would find gaps in our safety bounds

**Adoption cost:** Adapter — policies need to be expressed as FLUX bytecode

**Verdict:** Medium value if we adopt conservation-enforcer-rs, otherwise not applicable

---

#### 6. snapkit-flux-bridge

**What it is:** Translates `HarmonyGovernor` configs (friction thresholds, Hurst floors) into FLUX bytecode. One-way compiler.

**Maturity:** Prototype, 11 tests

**Concrete synergy:**
- **Pattern for our vessel.toml compiler** — we could compile `vessel.toml` to FLUX bytecode that enforces physical limits
- Fixes a MIDI mapping bug we might have in similar normalization code

**Adoption cost:** Research project — would need to write our own compiler for maritime safety rules

**Verdict:** Study the pattern, implement our own vessel.toml→bytecode compiler

---

### Óthismos Family

#### 7. óthismos ⭐

**What it is:** Mathematical framework for measuring "pressure" — the force a bounded system exerts against its constraints. Computes `Π = ‖Δθ‖` where `Δθ = desired_step - actual_step`.

**Maturity:** Working, 135 tests, rich documentation

**Concrete synergy:**
- **Detects when our envelope is actively constraining the playbook** — high pressure means the system wants to do something it can't
- Molt cycle classification (Expansion → Resistance → Crisis → Settlement → Dormancy) could detect when our safety limits are too tight
- Popcorn diagnostic (Pop/Burn/Seep/Dormant) could detect playbook malfunction (Burn = external pressure but no internal response)

**Adoption cost:** Adapter — need to define what "desired step" means for actuation (playbook intent vs. envelope-clamped command)

**Verdict:** **HIGH VALUE for observability** — pressure metrics are early warning of safety envelope issues

---

#### 8. othismos-llm

**What it is:** LLM-specific pressure measurement — how much output changes when context is truncated. JSD, KL, cosine distance metrics.

**Maturity:** Working

**Concrete synergy:**
- Could be used if we add LLM-based playbook authoring — measure how much context affects generated control code

**Adoption cost:** Research project — we don't use LLMs in the control loop yet

**Verdict:** Low value for current architecture, bookmark for future

---

### PLATO Family

#### 9. plato-core (Python)

**What it is:** Foundation types for PLATO ecosystem — TrainingTile, TileType, LamportClock, MeshRegistry for plugin discovery.

**Maturity:** Working

**Concrete synergy:**
- **Tile content-addressing** could be used for playbook versioning (already content-addressed by hash, but PLATO adds Lamport clocks)
- Mesh registry pattern could be used for driver discovery

**Adoption cost:** Drop-in for types, but we don't need the Python runtime

**Verdict:** Study the tile pattern, implement our own versioning

---

#### 10. plato-core-rs ⭐

**What it is:** Rust PLATO room implementation with sensors, actuators, alarms, history buffer, wire protocol. Text-based commands, JSON-line responses.

**Maturity:** Working, 40+ tests

**Concrete synergy:**
- **Room pattern maps directly to our vessel abstraction** — sensors (NMEA), actuators (rudder, throttle), alarms (overheat, shallow water)
- Wire protocol is human-typeable and LLM-parseable — could be our debugging/inspection interface
- History buffer (10,000 ticks) matches our black box requirements
- Alarms with cooldowns match our envelope alarm pattern

**Adoption cost:** **Drop-in** — could expose our vessel state as a PLATO room for telnet inspection

**Key insight:** This is **exactly our L4 agent interface** — a text protocol for reading state and triggering actuators. The difference is our bus is async/RPC, PLATO is synchronous/TCP.

**Verdict:** **HIGH VALUE** — adopt as our external debugging/inspection protocol

---

#### 11. plato-portal (SuperInstance SDK)

**What it is:** Python SDK for persistent multi-agent systems with memory and fleet coordination.

**Maturity:** SDK works, roadmap is aspirational

**Concrete synergy:**
- Agent memory pattern (markdown files) could be used for playbook provenance
- Fleet registry could be used for multi-vessel coordination

**Adoption cost:** Research project — most functionality is aspirational

**Verdict:** Low value for boat-agent, bookmark for fleet operations

---

### PLATO Rooms

#### 12. plato-room-code-review

**What it is:** Automated code review as PLATO room — heuristic checks (missing tests, secrets, CI bypass, SQL injection).

**Maturity:** Working, CI, GitHub Action ready

**Concrete synergy:**
- **Pattern for automated audits** — shows how to express domain checks as room sensors/actuators
- Conservation enforcer integration shows two-layer governance (room checks + meta-checks)

**Adoption cost:** Adapter — would need to write maritime-specific checks

**Verdict:** Study the pattern, implement our own audit room for playbook validation

---

#### 13. plato-room-security-audit

**What it is:** Security audit room — heuristic security checks (SQL injection, XSS, path traversal, hardcoded secrets).

**Maturity:** Working

**Concrete synergy:**
- Same as code-review room — pattern for domain-specific auditing

**Verdict:** Study the pattern

---

#### 14. plato-room-deployment-approval ⭐

**What it is:** Deployment gating room — CI status, test coverage, security scan results, rate limits, approval checks.

**Maturity:** Working

**Concrete synergy:**
- **Rate limiting as conservation law** — max deployments per day, enforced by room
- **Pattern for our human approval gate** — APPROVED stage requires human sign-off before promotion
- Demonstrates two-layer enforcement (room checks + FLUX conservation laws)

**Adoption cost:** Adapter — adopt the approval workflow, not the GitHub integration

**Verdict:** **HIGH VALUE** — study the approval gate pattern for our playbook stage-gate

---

## Key Architectural Comparisons

### FLUX Bytecode vs. Our Playbook

| Aspect | FLUX | boat-agent |
|--------|------|------------|
| **Representation** | Bytecode (`.bin`) | Python functions |
| **Determinism** | Guaranteed by VM | Structured, but not proven |
| **Auditability** | Disassemble to bytecode | Read source code |
| **Policy layer** | FLUX VM | Safety envelope |
| **Stage-gate** | Conservation enforcer | Playbook lifecycle |
| **Rollback** | Hash-based registry | Content-addressed playbooks |

**Key insight:** Our playbook lifecycle is **already similar** to FLUX policy enforcement. The difference is FLUX bytecode is provably deterministic; Python is only "probably deterministic."

### PLATO Room vs. Our Vessel

| Aspect | PLATO | boat-agent |
|--------|-------|------------|
| **Sensors** | Named readings | NMEA telemetry → bus |
| **Actuators** | Writable outputs | Intent → envelope → drivers |
| **Alarms** | Rule-based triggers | Envelope checks |
| **History** | Ring buffer | Black box append-only |
| **Protocol** | TCP text | In-process bus |

**Key insight:** PLATO rooms expose the same pattern we use internally, but over a network protocol. We could expose a PLATO interface for **external debugging and inspection**.

### Óthismos vs. Our Envelope

| Aspect | Óthismos | boat-agent |
|--------|----------|------------|
| **What it measures** | Pressure against constraints | Constraint violations |
| **Signal** | `Π = ‖Δθ‖` | Binary (allowed/rejected) |
| **Use case** | Training optimization | Real-time safety |
| **Output** | Scalar pressure, per-constraint breakdown | Rejection reason |

**Key insight:** Óthismos gives us a **gradient** of constraint pressure — we can see *how hard* the system is pushing against limits, not just *that* it's pushing.

---

## Top 3 Recommended Adoptions

### 1. conservation-enforcer-rs — Policy Audit Layer

**Value:** HIGH | **Cost:** ADAPTER

**What we get:**
- Bytecode representation of safety policies (provable, versioned, auditable)
- Zero-dep VM that can run inside our kernel
- Policy testing framework (flux-policy-tester) for envelope validation
- Audit logging that matches black box requirements

**How to adopt:**
1. Define maritime conservation laws (rudder rate, throttle rate, oscillation) as FLUX policies
2. Compile `vessel.toml` to FLUX bytecode
3. Run policy VM in parallel with envelope for redundancy
4. Use policy bytecode as the **authoritative representation** of safety limits

**Work required:**
- Write maritime policy definitions (FLUX assembly or `.flux.md`)
- Build `vessel.toml` → FLUX compiler (study snapkit-flux-bridge)
- Integrate policy VM into envelope as optional verification layer
- Add policy bytecode to playbook manifest

**Risk:** Medium — FLUX VM is well-tested, but maritime policies need safety validation

---

### 2. óthismos — Constraint Pressure Monitoring

**Value:** HIGH | **Cost:** ADAPTER

**What we get:**
- Early warning of envelope pressure before violations occur
- Molt cycle detection to identify when safety limits are too tight/loose
- Popcorn diagnostic to detect playbook malfunction (Burn = no response to pressure)

**How to adopt:**
1. Define "desired step" as playbook intent, "actual step" as envelope-clamped command
2. Measure pressure at each tick (difference between intent and clamped command)
3. Log pressure metrics to black box alongside state snapshots
4. Use pressure trends for predictive maintenance (envelope tuning)

**Work required:**
- Integrate PressureGauge into envelope (measure each arbitration)
- Define constraint boundaries for measurement (rate limits, bounds)
- Add pressure metrics to VesselState and bus telemetry lane
- Implement pressure-based alarms (high pressure = review envelope limits)

**Risk:** Low — óthismos is pure math, well-tested, no safety-critical path

---

### 3. plato-core-rs — External Inspection Protocol

**Value:** MEDIUM | **Cost:** DROP-IN

**What we get:**
- Human-typeable protocol for vessel inspection (telnet, LLM-queryable)
- Room pattern that maps 1:1 with our vessel abstraction
- History buffer for post-mortem analysis (complements black box)
- Standard wire protocol for external tools

**How to adopt:**
1. Wrap VesselState as PLATO sensors (read-only)
2. Wrap jog lever as PLATO actuator (write-only)
3. Map envelope alarms to PLATO alarms
4. Expose PLATO room on TCP port for debugging

**Work required:**
- Implement PLATO room wrapper around our bus/state
- Add TCP server for PLATO wire protocol
- Map our types to PLATO sensor/actuator format
- Document PLATO interface for external tools

**Risk:** Low — PLATO is non-critical inspection interface, runs alongside kernel

---

## Not Recommended (With Caveats)

### flux-runtime / flux-core
**Verdict:** Not directly applicable. FLUX is designed for text/LLM governance, not real-time control. Could be useful for playbook authoring tools, but not safety path.

### plato-room-*
**Verdict:** Study the patterns, don't adopt code. Code review, security audit, deployment approval rooms show useful patterns for automated checks, but the code is GitHub-specific.

### snapkit-flux-bridge
**Verdict:** Study the compiler pattern. The actual code is for snapkit configs, but the one-way compiler pattern is useful for `vessel.toml` → bytecode.

### othismos-llm
**Verdict:** Not applicable unless we add LLM-generated playbooks.

---

## Implementation Roadmap

### Phase 1: Pressure Monitoring (1-2 weeks)
- Integrate óthismos PressureGauge into envelope
- Log pressure metrics alongside state snapshots
- Add pressure-based alarms to UI

### Phase 2: Policy Bytecode (3-4 weeks)
- Define maritime FLUX policies
- Build `vessel.toml` → FLUX compiler
- Integrate conservation-enforcer-rs as verification layer
- Add policy bytecode to playbook manifests

### Phase 3: PLATO Inspection (1-2 weeks)
- Wrap vessel as PLATO room
- Expose TCP inspection interface
- Add PLATO history buffer for debugging

---

## Conclusion

The FLUX/PLATO ecosystem has **three high-value components** for boat-agent:

1. **conservation-enforcer-rs** for policy auditability
2. **óthismos** for constraint pressure monitoring
3. **plato-core-rs** for external inspection

All three are **working, well-tested Rust code** that can be integrated with adapter-level work. The primary gap is defining maritime-specific semantics (what's a "conservation law" for rudder rate? what's "pressure" for throttle?).

The pattern across all three is **provable, auditable enforcement** — bytecode for policies, scalar metrics for pressure, text protocol for inspection. This aligns with our safety philosophy: the envelope is pure Rust, auditable, and reviewable.

**Recommendation:** Start with óthismos (lowest risk, immediate observability value), then add policy bytecode (higher value, medium risk), then add PLATO inspection (convenience, low risk).

---

**Appendix: Repository Status**

| Repo | Tests | Language | Status |
|------|-------|----------|--------|
| flux-runtime | 2037 | Python | ✅ Production |
| flux-core | 51 | Rust | ✅ Production |
| conservation-enforcer | 60+ | Python | ✅ Production |
| conservation-enforcer-rs | 150+ | Rust | ✅ Production |
| flux-policy-tester | Passing | Python | ✅ Working |
| snapkit-flux-bridge | 11 | Python | 🔧 Prototype |
| othismos | 135 | Python | ✅ Production |
| othismos-llm | Passing | Python | ✅ Working |
| plato-core | Passing | Python | ✅ Working |
| plato-core-rs | 40+ | Rust | ✅ Working |
| plato-portal | N/A | Python | 📖 Docs |
| plato-room-code-review | Passing | Python | ✅ Working |
| plato-room-security-audit | Passing | Python | ✅ Working |
| plato-room-deployment-approval | Passing | Python | ✅ Working |

---

**Generated:** 2026-07-19
**Analyst:** Claude (Opus 4.5)
**For:** boat-agent project
