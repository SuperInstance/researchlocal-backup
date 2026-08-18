# Navigation — The Cockpit Map

> **If you are a fresh agent or human looking for where to start,
> you are in the right place.** This document is the cockpit map: it
> tells you which folder is which specialist, what each one does,
> what its IO contract is, and how to dispatch work to it.
>
> For the *why* (git-agent paradigm, cockpit hierarchy, IO contract),
> see `docs/AGENT_OPERATING_MODEL.md`. For the *how* (operating rules),
> see `docs/BOOTCAMP.md`. This file is the *where*.

---

## TL;DR — the cockpits

| Cockpit | Purpose | Owner | Status |
|---|---|---|---|
| `tzpro-agent/` | Captain's cockpit. Plans, doctor, harness, dashboard, tray, capture | captain (this repo) | Phase 1 done, Phase 2 next |
| `tzpro-bridge-agent/` | NMEA COM6 -> TCP/HTTP/JSON bridge. Not yet a separate repo. | future specialist | design only |
| `tzpro-cascade-agent/` | Vision cascade: hourly/decaminute/daily loops over captures. Lives as `cascade/` here. | cascade specialist | running in `cascade/` |
| `tzpro-monologue/` | Ship-metaphor internal-monologue agent (self-data-miner). Not yet scaffolded. | monologue specialist | design only |
| `tzpro-cloudflare/` | Cloudflare Worker sync (Phase 8). | cloud-sync specialist | wrangler installed, unauthed |
| `forks/<repo>/` | Cloned third-party repos, each its own cockpit for a sub-specialist | varies | ad-hoc |

Each row is a **separate cockpit**. To become the specialist of any
cockpit, `cd` into its folder and run `claude`. To dispatch work to
it from the captain's seat, use the dispatch recipe below.

---

## The captain's cockpit: `tzpro-agent/`

**You are here.** This is the captain's cockpit. It contains the
doctor, the harness, the dashboard, the tray app, the capture daemon,
and the cascade loops (which would later move to their own cockpit).

### Onboard order (read in this sequence)

```
python delegation/onboard.py              # default; prints the read order
```

The default scope reads these files in order:

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
docs/PLANS/weekly/<THISWEEK>.md
docs/BOOTCAMP.md
docs/BATON_PASS_<latest>.md
docs/ROADMAP.md
```

After reading, run `python doctor.py check` to confirm the system is
healthy. Pick up the NEXT bullet from the daily plan.

### Focused scopes (read fewer files, faster)

Run `python delegation/onboard.py <scope>`:

| Scope | Use when | Files (relative to WORKSPACE) |
|---|---|---|
| `default` | resuming work | plans + bootcamp + baton + roadmap |
| `phase-1` | verifying/extending Phase 1 (bridge/dashboard/tray/capture) | doctor, dashboard, tray_app, capture_daemon, claude_runner, HARDWARE_SETUP |
| `phase-2` | wiring the analyzer (next major work) | plans + ROADMAP + BOOTCAMP |
| `monologue` | building `tzpro-monologue/` | plans + BOOTCAMP (rule 13) + BATON_PASS |
| `cost-throttle` | building the cloud-cost throttle | plans + delegation/budget.py + BOOTCAMP (rule 10) |
| `debugging` | something's broken, find the right doctor check | plans + doctor.py + TROUBLESHOOTING |
| `harness` | understanding/extending the delegation harness | all of `delegation/` + BOOTCAMP + NAVIGATION |

Add a new scope by appending a `## scope: <name>` block at the
bottom of this file. The reader is `delegation/onboard.py`.

### Dispatch from the captain's seat

```powershell
# Spawn a one-shot Claude Code in this repo for a focused review task
python delegation/claude_runner.py `
    --task delegation/tasks/<task_name>.json

# Run the harness self-test
python delegation/_test_harness.py

# See what previous-you thought should happen next
python delegation/next_action.py --scope daily
python delegation/next_action.py --scope phase-2
```

---

## Specialist cockpits (how to dispatch)

A specialist cockpit is any folder where you can spawn a fresh
Claude Code session and get a useful agent out of it. The git-agent
paradigm: **the folder *is* the specialist's memory.**

### Dispatch recipe (general)

```powershell
# 1. Enter the cockpit
cd <specialist-folder>

# 2. (Optional) Read its AGENT.md to understand its job
cat AGENT.md    # if present

# 3. Spawn Claude Code with a specific task
claude --print "<task description>. Output JSON to stdout."

# 4. Parse the JSON in the captain's session and act on it
```

### Specialist: `cascade/` (within `tzpro-agent/`)

This is a *proto-cockpit* — a sub-folder that already behaves like a
specialist. It contains the vision cascade loops:
`daemon.py`, `daily_loop.py`, `decaminute_loop.py`, `hourly_loop.py`,
`retention.py`. To spawn a Claude session as the cascade specialist:

```powershell
cd tzpro-agent/cascade
claude --print "Review the cascade loops. Propose Phase-2 analyzer
                wiring (small-model-every-10-min, escalate-to-heavy
                for selected moments). Output to delegation/tasks/
                cascade-phase2.json"
```

### Specialist: `tzpro-monologue/` (future, not yet scaffolded)

This will be a sibling repo containing the ship-metaphor
internal-monologue agent. When you scaffold it, the bootstrap order
will be:

```
tzpro-monologue/
  AGENT.md                              # the specialist's charter
  BOOTCAMP.md                           # the specialist's rules
  docs/PLANS/current.md                 # the specialist's plan
  hull/keel/AGENT.md                    # the constitution
  hull/plating/                         # the persisted self (git-trackable)
  slip/                                 # where new construction happens
  chandlery/                            # tool library
  log/                                  # the captain's log (the boat's log)
  monologue.py                          # the entry point
  cost/throttle.py                      # per-rule-10 cost gaming
  delegation/                           # the specialist's harness
```

**To dispatch:** `cd tzpro-monologue && claude --print "..."`. The
specialist will read its own `AGENT.md` first and operate within its
own rules. The captain's cockpit reads its outputs via JSON.

### Specialist: forks

When you find a third-party tool you want as a specialist, clone it:

```powershell
cd forks
git clone https://github.com/some-cool/marine-lib.git
cd marine-lib
claude --print "You are the marine-lib specialist. Summarize the IO
                contract and write a JSON spec to ../delegation/tasks/
                marine-lib-spec.json"
```

The clone is now a cockpit. The Claude session that worked on it
left a JSON spec and modified files behind; the *next* Claude
session spawned in there inherits the context via the files.

---

## The dispatch taxonomy (when to use which IO)

| Task type | Best medium | Why |
|---|---|---|
| Big review (>500 lines) | `delegation/claude_runner.py` -> JSON spec | parallelizable, parseable |
| Test scaffold | `delegation/claude_runner.py` -> JSON | structured test list |
| Doc draft (>500 words) | `delegation/claude_runner.py` -> markdown | human-readable draft |
| Small edit (<50 lines) | direct `edit_file` | faster than spawning |
| One-shot refactor | direct + commit | easy to revert |
| Specialist research | `claude --print` in a clone cockpit | specialist inherits context |
| Cost-bounded work | cloud API via provider + `cost/throttle.py` | measurable |

When in doubt, **start small** (direct edit) and **escalate**
(claude_runner.py or specialist clone) only when the work is too big
for your context window. The harness is the yoke; the model reaches
for it.

---

## Cross-cockpit communication

Specialists and the captain communicate through files in **shared
locations**:

| Path | Used for |
|---|---|
| `tzpro-agent/delegation/tasks/*.json` | task specs (captain -> specialist) |
| `tzpro-agent/delegation/tasks/*.response.json` | specialist outputs (specialist -> captain) |
| `tzpro-agent/.tzpro-agent/budget.json` | shared budget state |
| `tzpro-agent/docs/DECISIONS.md` | committed decisions (read by all cockpits) |
| `tzpro-agent/vessel_state.jsonl` | the canonical vessel state stream |

There is no message bus, no socket, no RPC. Files. The same way git
itself works.

---

## Adding a new cockpit

When you (the agent) discover a new specialist is needed:

1. **Create the folder** as a sibling of `tzpro-agent/` (e.g.
   `tzpro-cascade-agent/`).
2. **`git init` inside it.**
3. **Write `AGENT.md`** — the specialist's charter (job, IO
   contract, success criteria).
4. **Write `BOOTCAMP.md`** — link to the parent's BOOTCAMP plus
   any cockpit-specific rules.
5. **Copy the `delegation/` harness** (budget, lesson_plan, onboard,
   claude_runner) — it's small, reusable, and gives the new cockpit
   zero-shot navigability for free.
6. **Add a row** to the cockpit table at the top of this file.
7. **Commit and push** so the captain's cockpit can dispatch to it.

This is the git-agent equivalent of standing up a new microservice.
The unit of deployment is the repo; the unit of operation is the
folder.

---

## See also

- `docs/AGENT_OPERATING_MODEL.md` — the doctrine
- `docs/BOOTCAMP.md` — the operating rules
- `docs/PLANS/current.md` — the active plan pointer
- `docs/BATON_PASS_<latest>.md` — the most recent handoff
- `docs/ROADMAP.md` — the phase index
- `docs/DECISIONS.md` — committed decisions

---

## Scope table (read by `delegation/onboard.py`)

This section is machine-readable. `delegation/onboard.py` parses
the `## scope: <name>` blocks below and uses them as the scope
table. The prose table above is for humans; this section is for
the harness.

To add a new scope, append a `## scope: <name>` block following
the format below. The summary line is required; the file list is
required.

## scope: default

summary: Resume the boat-agent project at the latest checkpoint under the git-agent paradigm.

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
docs/PLANS/weekly/<THISWEEK>.md
docs/AGENT_OPERATING_MODEL.md
docs/BOOTCAMP.md
docs/NAVIGATION.md
docs/BATON_PASS_2026-07-24.md
docs/ROADMAP.md
```

## scope: phase-1

summary: Verify or extend Phase 1 (bridge, dashboard, tray, capture_daemon).

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
doctor.py
dashboard.py
tray_app.py
capture_daemon.py
delegation/claude_runner.py
docs/HARDWARE_SETUP.md
```

## scope: phase-2

summary: Wire the analyzer (next major work). Small model on 10-min cadence, escalate to heavy for selected moments.

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
docs/AGENT_OPERATING_MODEL.md
docs/BOOTCAMP.md
docs/ROADMAP.md
docs/BATON_PASS_2026-07-24.md
cascade/
delegation/budget.py
```

## scope: monologue

summary: Scaffold tzpro-monologue, the ship-metaphor internal-monologue agent.

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
docs/AGENT_OPERATING_MODEL.md
docs/BOOTCAMP.md
docs/BATON_PASS_2026-07-24.md
```

## scope: cost-throttle

summary: Build the time-aware cloud free-tier cost throttle.

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
delegation/budget.py
docs/BOOTCAMP.md
```

## scope: debugging

summary: Something is broken; find the right doctor check.

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
doctor.py
docs/TROUBLESHOOTING.md
```

## scope: harness

summary: Understand or extend the delegation harness (budget, plans, onboard, claude_runner, next_action).

```
delegation/__init__.py
delegation/budget.py
delegation/lesson_plan.py
delegation/onboard.py
delegation/claude_runner.py
delegation/next_action.py
docs/BOOTCAMP.md
docs/NAVIGATION.md
```

## scope: dispatch

summary: Learn how to dispatch work to specialist cockpits (the git-agent spawn pattern).

```
docs/PLANS/current.md
docs/AGENT_OPERATING_MODEL.md
docs/NAVIGATION.md
delegation/claude_runner.py
```

## scope: doctrine

summary: Read the doctrinal documents (who you are, how to operate, where things live).

```
docs/AGENT_OPERATING_MODEL.md
docs/BOOTCAMP.md
docs/NAVIGATION.md
docs/PLANS/current.md
```

## scope: projection

summary: Understand or extend the projection layer (the IDE-shaped human surface: four panels, widget model, layout presets).

```
docs/PLANS/current.md
docs/PLANS/daily/<TODAY>.md
docs/AGENT_OPERATING_MODEL.md
docs/architecture/PROJECTION_LAYER.md
docs/architecture/DUAL_REPRESENTATION.md
docs/architecture/CAPTURE_PIPELINE.md
dashboard.py
```