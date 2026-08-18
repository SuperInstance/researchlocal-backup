# Beta Diary — Samira, DevOps/SRE

**Who I Am:** I'm Samira. I run 50+ microservices on Kubernetes for a mid-stage startup. My day job is keeping the lights on — PagerDuty, Prometheus, Grafana, the whole observability stack. I've evaluated maybe two dozen "innovative" monitoring tools over the past three years. Most of them are solutions looking for problems. I came into this skeptical. Here's what I actually found.

---

## Per-Crate Reviews

### cathedral-probe (6/10)

This is the one that caught my eye. The idea: treat your microservice topology as a graph, compute spectral properties (Laplacian eigenvalues, Fiedler value, Cheeger constant), and get a mathematical measure of how "healthy" the connectivity between services is.

**What's good:** The README is genuinely excellent. Within 30 seconds I understood what Fiedler value means in ops terms: "second-smallest eigenvalue of the graph Laplacian, higher = better connected, zero = disconnected." That's a clean, quantifiable health metric I could actually put on a dashboard. The Cheeger constant for bottleneck detection maps directly to something I care about — "which single link failure would hurt the most?" The API is dead simple: name your nodes, add edges with weights, call methods. Zero dependencies, `no_std` compatible. 500 lines of code, 26 tests all passing.

**What's wrong:** The tests are all toy graphs — triangles, lines, stars, a 20-node chain. That's fine for correctness, but I need to see this handle a real service mesh with 200+ nodes, weighted edges based on actual request rates, and streaming updates. Right now you rebuild the whole graph from scratch each time. No incremental updates, no streaming API, no Prometheus exporter, no Kubernetes CRD. The QR iteration algorithm for eigenvalue computation is O(n³) — that's going to hurt at scale. No benchmarks at all.

**The hard question:** Could I explain this to my on-call team at 3AM? "The Fiedler value dropped below 0.1" — I'd need to build an entire mental model translation layer. "Your service mesh is fragmenting" would work. But I'd need to build that translation myself.

**Ops viability:** Conceptually interesting. Needs a Prometheus exporter and incremental graph updates to be useful in production. As a library, it's clean and correct. As a monitoring tool, it's a prototype.

---

### crackle-runtime (5/10)

Pattern detection across task outputs. You fire tasks, each producing named metrics, then "cool the kiln" to discover clustering, phase transitions, conservation laws, and correlations.

**What's good:** The pottery metaphor is memorable. The API is clean — implement `CrackleTask`, fire tasks into a `Kiln`, call `cool()`, get patterns back. The `ThermalProfile` concept (fast/slow/no detection) is a smart way to control sensitivity. Zero async, no background threads, pure synchronous. The pattern descriptions are human-readable. 72 tests pass.

**What's wrong:** One of the examples (`ci_patterns.rs`) doesn't compile — `PatternKind` has no `to_uppercase()` method. That's a trivial bug, but it means nobody ran the examples before shipping. The pattern detection is extremely basic: Euclidean distance for clustering, Pearson correlation, first-half vs second-half mean comparison for phase transitions. These are statistics 101 level. For a real incident — say, a gradual latency degradation across 12 services over 45 minutes — I don't see how this catches it. There's no time-series awareness, no seasonality handling, no concept of baseline vs anomaly. The "emergent pattern detection" marketing oversells what's actually happening: simple statistical tests on metric vectors.

**In a hot path?** The README says "no overhead during your hot path" because detection runs after. That's true. But you still need to fire tasks, record metrics, and store them. For 10,000 requests/second, that's 10,000 task objects in memory before cooling. No backpressure, no streaming, no windowing.

**The honest take:** This would catch obvious outliers in a small batch of homogeneous tasks. It would not catch the subtle, time-dependent incidents that actually wake me up at 3AM.

---

### conservation-checker (7/10)

The most pragmatically useful of the bunch. Track quantities that must not decrease (or increase, or stay conserved) with tolerance and phase detection.

**What's good:** This maps directly to real ops problems. Rate limiting tokens? Budget tracking? Token consumption against a cap? All of these are conservation laws. The API is exactly right: `register` a quantity with a tolerance, `update` it as your system runs, `snapshot` to record history, check `violations()`. The phase detection (Stable → PreTransition → Transitioning → Resolving) is actually useful — "your API rate limit is approaching" is a real alert I'd want. 53 tests pass. Examples work. Zero dependencies.

**What's wrong:** The phase detection is simplistic — it's basically looking at drift rate and current violation status. No statistical significance testing. "PreTransition" fires when drift is accelerating, but how do you distinguish "accelerating toward a real problem" from "normal variance"? I'd want confidence intervals or at least a configurable lookback window. The `reset_baseline()` method is dangerous without audit logging — if you reset baseline after a violation, you've just hidden the problem.

**Would I use it?** Honestly, yes, for budget enforcement in a batch processing pipeline. Not for real-time alerting — for that I'd need it behind a metrics endpoint with Prometheus scraping. But as an embedded invariant checker inside a service? Absolutely.

---

### negative-space-testing (4/10)

"Test what your system does NOT do." It bundles SpaceMap (forbidden output zones), NegativeTest (per-output exclusion rules), ConservationChecker (re-exported from the standalone crate), CracklePhase (deferred assertions), and CathedralProbe (inter-component relationship tests).

**What's good:** The philosophical framing is interesting. Testing negative space — "the pattern lives in the holes" — is a genuine insight. In ops, half our alerts ARE negative space: "no 5xx errors", "no PII in responses", "no service should be unreachable."

**What's wrong:** It's four loosely related tools duct-taped into one crate. The ConservationChecker here is a simpler version of the standalone crate — why would I use this one instead of the real one? The CathedralProbe component is NOT the same as the standalone cathedral-probe — it's a simpler relationship checker. The naming collision is confusing. CracklePhase (deferred assertions with "kintsugi" outcomes) is conceptually interesting but I can't think of when I'd use it in production — it feels like a testing framework abstraction, not an ops tool.

**For my team:** The README is well-written but the concept is too abstract. "Define what your system doesn't do" — my team would say "that's just assertions and monitoring." They're not wrong. This is a testing framework, not an ops tool, and it should be marketed as such.

---

### hermes-construct (the platform) (5/10)

This is the full platform — a "tile-operating shell kernel" that orchestrates LLM agents with conservation budgets, ensigns (cheap watch models), rooms, tiles (tasks), and ports (inter-room communication).

**Deployment scripts:** Both `deploy-oracle.sh` and `deploy-jetson.sh` are well-structured, idempotent, with proper error handling, color output, and architecture detection. They install system deps, build from source, create a dedicated user, set up directories, and install the systemd service. This is genuinely competent deployment scripting.

**systemd service file:** Here's where I have strong opinions. The hardening is GOOD:
- `NoNewPrivileges=true` ✅
- `ProtectSystem=strict` ✅
- `ProtectHome=true` ✅
- `PrivateTmp=true` ✅
- `ReadWritePaths=/var/lib/hermes-construct` (only the state dir) ✅
- `ProtectKernelTunables=true` ✅
- `ProtectKernelModules=true` ✅
- `ProtectControlGroups=true` ✅
- `RestrictSUIDSGID=true` ✅
- `RestrictNamespaces=true` ✅
- `LockPersonality=true` ✅

**Missing from the service file:**
- No `CapabilityBoundingSystem=` / `AmbientCapabilities=` restriction
- No `SystemCallFilter=` or `SystemCallArchitectures=` 
- No `PrivateDevices=true`
- No `NetworkNamespacePath=` or port binding restrictions
- `TimeoutStopSec=30` might be tight for graceful shutdown with SQLite WAL checkpoint
- No `WatchdogSec=` for health checking
- No `LogRateLimitIntervalSec=` / `LogRateLimitBurst=` (an ensign storm could flood journald)
- `StateDirectoryMode=0750` — why not `0700`? The hermes user should own this exclusively.

The `KillSignal=SIGINT` with graceful shutdown is a nice touch.

**Ensign configs:** The escalation model (cheap model watches → expensive model analyzes → human decides) is genuinely clever. The cost model makes sense: $0.001-0.015/check for ensigns, $0.05-0.50 for escalation. The rate limiting on escalation (`max_escalations_per_hour: 5`) prevents runaway costs. The JSON configs are clean and simple.

**But:** This is an LLM agent orchestrator, not a monitoring tool. The conservation budget, ensign watch, and pattern detection concepts are embedded inside a framework that's fundamentally about running AI agents. If I just want the monitoring/invariant-checking parts, I can't extract them without buying into the whole agent runtime.

**Dependencies:** hermes-construct pulls in tokio, rusqlite, reqwest, teloxide, and a bunch more. That's a real service, not a library. Fine for what it is, but heavy.

---

## Deployment & Ops Readiness: 5/10

| Concern | Score | Notes |
|---------|-------|-------|
| systemd hardening | 7/10 | Good baseline, missing syscall filtering and device isolation |
| Deployment scripts | 8/10 | Idempotent, well-structured, proper error handling |
| Observability | 2/10 | No metrics endpoint, no Prometheus exporter, no structured logging format |
| Configuration | 5/10 | Env files for secrets (ok), no config validation, no hot-reload |
| Graceful shutdown | 6/10 | SIGINT handler exists, but 30s timeout might be tight |
| Health checking | 2/10 | No `/healthz`, no watchdog, no liveness probe |

---

## Integration Story: 3/10

**This is the biggest disappointment.** The standalone crates (cathedral-probe, crackle-runtime, conservation-checker, negative-space-testing) are NOT dependencies of hermes-construct. The hermes-construct platform reimplements conservation tracking internally (`src/conservation.rs`) without using the standalone crate. The cathedral-probe and crackle-runtime concepts exist in hermes-construct as philosophical inspirations but not as actual code dependencies.

There's no unified data format between crates. Cathedral-probe outputs eigenvalues; crackle-runtime outputs pattern structs; conservation-checker outputs violation lists. None of these feed into each other. If I wanted to:
1. Monitor my topology with cathedral-probe
2. Detect anomalies with crackle-runtime  
3. Track budgets with conservation-checker
4. Run it all on hermes-construct

...I'd be writing all the glue code myself. The crates share a philosophical framework but not a technical one.

---

## What Genuinely Surprised Me (Positive)

1. **The Fiedler value concept is actually useful.** I've been doing SRE for 6 years and nobody's ever shown me a single number that captures "how healthy is the connectivity of my service mesh." This might be genuinely novel for ops.

2. **The ensign escalation model is clever.** Cheap models on constant watch, expensive models on demand, humans as final authority — this is how you'd actually want to run AI-assisted monitoring at scale without going bankrupt.

3. **Zero dependencies.** Cathedral-probe and conservation-checker have literally zero deps. In a world of left-pad incidents and supply chain attacks, that's a genuine advantage.

4. **The systemd service hardening** shows someone actually thought about running this in production, not just on a laptop.

5. **The conservation phase detection** (Stable → PreTransition → Transitioning → Resolving) maps to a real operational pattern. I've seen this exact trajectory in budget exhaustion incidents.

---

## Dealbreakers

1. **No Prometheus exporter anywhere.** I cannot adopt any monitoring tool that doesn't expose metrics in a format my existing stack can consume. Period.
2. **No Kubernetes integration.** No CRDs, no service discovery, no pod topology awareness. I run on K8s; this doesn't exist in my world.
3. **No streaming/incremental updates in cathedral-probe.** Rebuilding the entire graph on every topology change doesn't scale.
4. **The crates don't interop.** They're a philosophy, not a platform.
5. **crackle-runtime's example doesn't compile.** Ship quality matters.

---

## Would I Deploy This?

**Not today.**

cathedral-probe is the most promising piece — I'd prototype it as a sidecar that reads Kubernetes service topology and exposes Fiedler value / Cheeger constant as Prometheus gauges. If the numbers correlated with real incidents, I'd invest more.

conservation-checker I'd actually use as an embedded library in a budget-tracking service, but it needs audit logging and a metrics bridge.

hermes-construct as a platform is too opinionated for my infrastructure. It's an LLM agent runtime, not a monitoring tool. The monitoring concepts are embedded in something I don't need.

---

## Specific Recommendations

1. **Build a Prometheus exporter for cathedral-probe.** Something that reads Kubernetes Endpoints/Services, builds the graph, and exposes `cathedral_fiedler_value`, `cathedral_cheeger_constant`, `cathedral_component_importance{node="..."}` as gauges. That's a weekend project and it would make this immediately useful.

2. **Make the crates actually interop.** Define a shared `TopologySnapshot` format. Cathedral-probe produces it, crackle-runtime consumes it, conservation-checker tracks invariants on it. Right now they're four separate tools with matching aesthetics.

3. **Add incremental graph updates to cathedral-probe.** For a service mesh, the topology changes gradually (one deployment at a time). Don't recompute eigenvalues from scratch on every change.

4. **Fix the crackle-runtime example.** It's one line. `p.kind().to_uppercase()` → `format!("{}", p.kind()).to_uppercase()` or implement `Display` with uppercase. Trivial but it's the first thing people will try.

5. **Add structured JSON logging to hermes-construct.** For journald/Fluentd/Logstash integration. RUST_LOG=info isn't enough.

6. **Add syscall filtering and PrivateDevices to the systemd unit.** You're 90% there on hardening — finish the job.

7. **Write benchmarks.** I need to know what happens to cathedral-probe's eigenvalue computation at 200, 500, 1000 nodes before I even think about running it in production.

8. **Stop calling crackle-runtime "emergent pattern detection."** It's statistical aggregation. That's fine! Statistical aggregation is useful! But "emergent" implies something more sophisticated than Pearson correlation and Euclidean clustering.

---

*Diary written by Samira, DevOps/SRE, 2026-06-01. These are my honest opinions. I'd rather be wrong and specific than polite and useless.*
