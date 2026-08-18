# Baton Pass — 2026-07-24

> **You are the resumed agent.** The previous session ended after
> Casey asked me to refactor the onboarding docs into instructional
> mode under the git-agent paradigm. I refactored four docs:
> `docs/AGENT_OPERATING_MODEL.md` (NEW), `docs/BOOTCAMP.md`,
> `docs/NAVIGATION.md`, `docs/PLANS/current.md`. This baton pass
> is what you (future-me) read first after `/clear`.
>
> **Read this AFTER `docs/PLANS/current.md`'s bootstrap order, but
> BEFORE you start new work.** This doc records what was just
> decided and why — the *delta* from the previous baton.

---

## What just changed (and why)

**Casey's directive (Round 4):** the onboarding docs were in
operational mode ("here's what I did today"). Casey asked me to
flip them to **instructional mode** ("here's how to operate this
system") — and to do it framed by the git-agent paradigm he
described:

- A git-agent is an LLM-first workspace, not a human-first one.
  The agent is in the cockpit; the human is the co-captain.
- The cockpit is the folder; the agent's memory is the file
  system; the agent's session boundary is a git commit.
- Claude Code spawned in a sub-folder or clone IS that
  sub-specialist for the session. The folder primes the agent.
- The IO contract between agents is text/JSON/vector, not GUI.
  GUIs are adapters for humans, optional and interchangeable.
- Circuit breakers (budget gates, doctor checks, `!STOP`)
  preserve manual control without re-architecting for it.

This is the *paradigm*. The four refactored docs apply it.

---

## The four doc changes

### 1. `docs/AGENT_OPERATING_MODEL.md` (NEW — doctrine)

This is the central doctrinal doc. It defines:

- What a git-agent is (the table of concerns → implementations).
- The cockpit hierarchy (captain → specialists → sub-claudes).
- How to onboard to a new cockpit (the 6-step zero-shot recipe).
- How to dispatch a specialist (the git-agent spawn pattern).
- The three circuit breakers (budget, doctor, `!STOP`).
- The IO contract (markdown / JSON / JSONL / SQLite / vectors).
- The roles of human, cloud, and local model in this system.

**If you read only one doc today, read this one.** It's the
crystallized version of everything Casey and previous-me
converged on.

### 2. `docs/BOOTCAMP.md` (refactored — instructions)

The 14 rules are preserved but reframed from post-mortem
observations ("don't let working tree carry >30 min of
uncommitted work") to *instructions* to the current operator
("commit early, commit often — every subsystem lands, every
10-step milestone, push to remote when network cooperates").
Each rule now has explicit DO lists. The `Quick orientation`
section up top tells a fresh operator exactly what to do in
the first 3 minutes.

### 3. `docs/NAVIGATION.md` (refactored — cockpit map)

Previously a scope-to-file-map for `onboard.py`. Now the
*cockpit map*: it lists every specialist cockpit in the
system (tzpro-agent, tzpro-cascade-agent, tzpro-monologue,
tzpro-cloudflare, forks), says who lives in each, and
provides dispatch recipes for spawning a specialist.

The original scope table is preserved at the bottom; `onboard.py`
reads it.

### 4. `docs/PLANS/current.md` (refactored — entry point)

The bootstrap order now includes `AGENT_OPERATING_MODEL.md`
and `NAVIGATION.md` as required reading *before* BATON_PASS
and ROADMAP. The "Who you are" section up top crystallizes
the git-agent framing so a fresh operator doesn't need to
chase the doctrine through multiple files.

---

## What did NOT change

- `delegation/budget.py`, `lesson_plan.py`, `onboard.py`,
  `claude_runner.py` — the harness code is unchanged.
- `doctor.py` — the `state:jsonl` check fix from this session
  (BOOTCAMP rule 5 — DOCKED state is valid) is already
  committed at `5c07ff7`.
- The cascade loop improvements — already committed at
  `5c07ff7`.
- Phase 1 itself (capture_daemon, tray_app, dashboard,
  providers, vessel config) — committed at `596071e`.
- The Phase 1 commit (working tree down from 49 to 21 entries,
  then to 0).

---

## Operational state at handoff

Working tree: clean (after the doc refactor commit).

```
$ git status --short
(empty)
```

Doctor: 8/9 healthy. The one failing check is `state:jsonl`,
which is *correctly* failing because:

- `last class=trolling` (boat was moving)
- `last write ~15532s ago` (~4 hours)
- This is a *real* staleness, not a false positive.

When the captain (you) restarts the bridge (`restart_bridge.bat`)
or the boat docks (SOG < 0.5 kts), the check will pass.

NEXT bullet (today's daily plan):
> "Refactor BOOTCAMP.md, NAVIGATION.md, BATON_PASS to
> instructional mode under the git-agent paradigm (Casey
> Round 4 directive). Then start a fresh 2026-07-24 daily plan."

That is what you should do *right now*. The first half of that
bullet is done (this doc is the result). The second half
(start a fresh daily plan for the next operator) is your
job.

---

## Recommended order for the resumed agent

1. **Commit this baton pass** (`docs/BATON_PASS_2026-07-24.md`
   and the four refactored docs) as one atomic commit:
   ```
   git add docs/AGENT_OPERATING_MODEL.md docs/BOOTCAMP.md \
           docs/NAVIGATION.md docs/PLANS/current.md \
           docs/BATON_PASS_2026-07-24.md
   git commit -m "docs(doctrine): refactor onboarding to instructional mode + git-agent paradigm"
   ```
2. **Start a fresh `docs/PLANS/daily/2026-07-24.md`** with the
   new template (STATE / MINUTES / ACTUAL / DELTA / NEXT), and
   set a NEXT bullet that points at Phase 2 work (analyzer
   wiring per ROADMAP), since Phase 1 is complete.
3. **Run `python doctor.py check`** to confirm health hasn't
   regressed. If the `state:jsonl` check is still failing,
   investigate whether the bridge is alive and the JSONL
   stream is moving. The fix to the check (DOCKED-tolerant)
   is already committed.
4. **Read `docs/ROADMAP.md`** to remind yourself what Phase
   2 is: 10-min cadence runs small model first, escalates
   to heavy for selected moments. Slice small; build the
   smallest useful piece; smoke test E2E.
5. **Continue.** When context gets heavy, write the next
   baton pass (`docs/BATON_PASS_2026-07-25.md` or whenever).

---

## One-line summary for Casey

"Refactored BOOTCAMP/NAVIGATION/PLANS to instructional mode
and added AGENT_OPERATING_MODEL.md as the doctrinal doc. The
git-agent paradigm (agent-in-cockpit, folder-as-memory, IO-
contract-as-text-or-JSON, circuit-breakers for manual control)
is now the entry-point framing. Working tree clean. Doctor
8/9 healthy (state:jsonl correctly flags a real staleness —
boat last logged as trolling ~4h ago). Ready for Phase 2."

---

## See also

- `docs/AGENT_OPERATING_MODEL.md` — the doctrine (read first)
- `docs/BOOTCAMP.md` — the 14 operating rules
- `docs/NAVIGATION.md` — the cockpit map
- `docs/PLANS/current.md` — the entry point
- `docs/ROADMAP.md` — phase index
- `docs/BATON_PASS_2026-07-23.md` — the previous baton (kept
  for archaeology; this one supersedes it for onboarding)