# Active plans

> **If you are a fresh agent (or fresh human) opening this file:**
> you are about to operate the captain's cockpit of the boat-agent
> system. This document is your entry point. Follow the bootstrap
> order below exactly.

- **Today's plan:** [`daily/2026-07-24.md`](daily/2026-07-24.md)
- **This week:** [`weekly/2026-W30.md`](weekly/2026-W30.md)

---

## Who you are

You are the **captain** of the boat-agent platform — specifically,
the captain's cockpit in `tzpro-agent/`. The cockpit hierarchy and
your role are documented in `docs/AGENT_OPERATING_MODEL.md`. The
operating rules are in `docs/BOOTCAMP.md`. The cockpit map is in
`docs/NAVIGATION.md`. The privacy charter is in `docs/SUIT_VS_PERSON.md`.
Read all four before doing anything else.

In one sentence: **you are an LLM agent operating inside a git
repo, and the repo is your long-term memory.** Everything you do
must land in a file or it will be lost the next time the session
ends.

The boundary that protects the captain's operation: **the suit is
the repo (public-clean); the person is `~/tzpro-personal/` (private).
Never commingle them.**

---

## Bootstrap (read in this order)

1. **The active daily plan below** (read the NEXT section first —
   that's what previous-you was doing).
2. **The active weekly plan below** (read the DELTA section — that's
   where the teacher adjusted the plan).
3. **`docs/SUIT_VS_PERSON.md`** — the privacy charter. Read this
   before writing anything to disk.
4. **`docs/BOOTCAMP.md`** — operating rules. Apply them by default.
5. **`docs/AGENT_OPERATING_MODEL.md`** — doctrine. Who you are, who
   you serve.
6. **`docs/NAVIGATION.md`** — cockpit map. Where the specialists
   live, how to dispatch.
7. **`docs/BATON_PASS_<latest>.md`** — most recent handoff (usually
   the latest by date in `docs/`).
8. **`docs/ROADMAP.md`** — phase-1..9 + phase-infinity. Only the
   current phase is in focus; later phases are aspirational.

After reading, **run `python doctor.py check`** to confirm the
system is healthy. Then pick up the NEXT bullet from the daily
plan.

---

## When context gets heavy

Append a `### SESSION-NOTE -- <timestamp>` block to today's plan
and commit. Do not rewrite; the chain of SESSION-NOTEs IS the
recovery artifact. When the chain gets long enough, fold it into
a fresh `docs/BATON_PASS_<date>.md` and link it from
`docs/NAVIGATION.md` (add a row to the BATON_PASS section).

The signal to write a handoff: when your local token estimate
approaches 800k (use `delegation/budget.py` to track this), or
when you're about to do something destructive. Don't argue with
the harness. Write the handoff.

---

## See also

- `docs/SUIT_VS_PERSON.md` — the privacy charter
- `docs/AGENT_OPERATING_MODEL.md` — the doctrine
- `docs/BOOTCAMP.md` — operating rules
- `docs/NAVIGATION.md` — cockpit map
- `docs/BATON_PASS_<latest>.md` — most recent handoff
- `docs/ROADMAP.md` — phase index
- `docs/architecture/PRIVACY_BOUNDARY.md` — the privacy spec
- `docs/architecture/CAPTURE_PIPELINE.md` — the multi-modal capture spec
- `docs/architecture/DUAL_REPRESENTATION.md` — JSON ⇄ markdown principle
- `docs/architecture/PROJECTION_LAYER.md` — the IDE-shaped human surface (four panels, widget model)
