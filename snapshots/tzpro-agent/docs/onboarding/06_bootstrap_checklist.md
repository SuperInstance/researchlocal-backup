# Bootstrap Checklist — The Next Agent's First Day

> **Print this. Tape it next to the laptop. Check off each item.**
> Total time: ~2 hours from cold start to fully operational understanding.

---

## Part 1 — Read first (90 minutes)

- [ ] **`docs/SUIT_VS_PERSON.md`** — the privacy charter. (30 min)
  - [ ] Confirm you understand: repo = suit, `~/tzpro-personal/` = person.
  - [ ] Confirm you understand: secrets in vault.dat, never in repo.
  - [ ] Confirm you understand: example data in `tests/`/`fixtures/` only.

- [ ] **`docs/BOOTCAMP.md`** — the 14 operating rules. (20 min)
  - [ ] Internalize: "commit early, commit often."
  - [ ] Internalize: "if the captain tells you X, do X, even if docs say Y."

- [ ] **`docs/onboarding/README.md`** — the reading order. (5 min)
  - [ ] Note that this directory is the handoff.

- [ ] **`docs/onboarding/01_session_boot.md`** — cold-boot ritual. (5 min)
  - [ ] Internalize the 5-step ritual: orient → doctor → capture → GPS → git.

- [ ] **`docs/onboarding/02_state_of_the_system.md`** — what's working. (10 min)
  - [ ] Note P0 (GPS) and P1 (capture) are done and durable.
  - [ ] Note P2 (analyzer) is blocked on missing `hermes_ensemble.py`.

- [ ] **`docs/onboarding/03_decisions_log.md`** — why we chose what we chose. (15 min)
  - [ ] Skim the 14 decisions (D-001 through D-014).
  - [ ] Pay attention to D-007 (ship capture before analyzer).
  - [ ] Pay attention to D-002 (stampfile singleton).

- [ ] **`docs/onboarding/04_paradigm_and_principles.md`** — the philosophy. (5 min)
  - [ ] Internalize: "every day this computer runs is a day of valuable data."

**Total reading time: ~90 minutes.** Take a break.

---

## Part 2 — Verify the system is actually working (10 minutes)

Run these commands. Each should succeed.

- [ ] **Doctor check:**
  ```powershell
  python doctor.py check
  ```
  Expected: `9/9 healthy`. If not, run `python doctor.py fix --yes`.

- [ ] **Capture verify:**
  ```powershell
  python capture_daemon.py verify
  ```
  Expected: `VERDICT: HEALTHY` with `Latest capture: <300s ago`.

- [ ] **GPS test:**
  ```powershell
  Test-NetConnection -Port 6006 -InformationLevel Quiet
  ```
  Expected: `True`.

- [ ] **Recent captures exist:**
  ```powershell
  Get-ChildItem captures/v3/2026-*/*.png | Sort-Object LastWriteTime -Descending | Select-Object -First 3
  ```
  Expected: 3 .png files with timestamps from today or yesterday.

- [ ] **Git state:**
  ```powershell
  git status --short
  ```
  Expected: empty (clean working tree). If not, read the uncommitted changes before doing anything else.

- [ ] **Dashboard reachable:**
  ```powershell
  Test-NetConnection -Port 8090 -InformationLevel Quiet
  ```
  Expected: `True`. Open `http://localhost:8090` in a browser to confirm.

**If all six pass: you are operational. Continue.**

---

## Part 3 — Understand the layout (15 minutes)

Skim the directory tree. Don't read every file — just know they exist.

- [ ] **`docs/`** — doctrine, plans, architecture, research.
  - [ ] `docs/engineer/` — subsystem reference (15 files).
  - [ ] `docs/onboarding/` — this handoff.
  - [ ] `docs/phases/` — one file per roadmap phase.
  - [ ] `docs/PLANS/daily/YYYY-MM-DD.md` — daily session notes.

- [ ] **Core Python modules at root:**
  - [ ] `nmea_bridge.py` — GPS reader.
  - [ ] `capture_daemon.py` — TZ Pro watchdog.
  - [ ] `capture_v3.py` — sounder screenshot.
  - [ ] `tray_app.py` — system tray UI.
  - [ ] `dashboard.py` — Starlette web UI on :8090.
  - [ ] `doctor.py` — health check + auto-repair.
  - [ ] `vault.py` — DPAPI secret store.
  - [ ] `db.py` — SQLite moments store.

- [ ] **Subsystems:**
  - [ ] `providers/` — 7 LLM backends (Ollama, OpenAI, DeepInfra, etc).
  - [ ] `schema/` — Moment, Anomaly, Correlation dataclasses.
  - [ ] `cascade/` — minute/hourly/daily loops (not yet supervised).
  - [ ] `memory/` — content-addressed blobs + SQLite index.
  - [ ] `analyst/`, `companion/`, `delegation/` — subsystem modules.

- [ ] **Runtime artifacts:**
  - [ ] `captures/v3/` — sounder screenshots, organized by date+position.
  - [ ] `vessel_state.jsonl` — NMEA append-only log.
  - [ ] `cascade_out/` — briefings, heartbeats.
  - [ ] `.vessel/bottles/` — inter-agent messages.
  - [ ] `logs/` — daemon logs.

**Total layout survey: ~15 minutes.**

---

## Part 4 — Know where the recovery tools are (5 minutes)

- [ ] **Desktop shortcut:** `TZ Pro Agent Tray.lnk` (OneDrive Desktop).
  Launches the tray. Tray auto-starts capture daemon.
- [ ] **Desktop shortcut:** `Fix TZ Pro Position.lnk` (local Desktop).
  Runs `fix_priority0.bat` — kills stale, restarts bridge + dashboard.
- [ ] **Tray menu items:**
  - [ ] Start/Stop Capture
  - [ ] Open Dashboard
  - [ ] Status (with freshness telemetry)
  - [ ] Open today's captures folder
  - [ ] Open latest capture
  - [ ] Verify capture health

**Total tool survey: ~5 minutes.**

---

## Part 5 — Understand your first duty (10 minutes)

The next agent's first duty is **the same as the previous agent's first duty**: keep P0 (GPS) and P1 (capture) working every day.

- [ ] **P0 must work:** if GPS drops, the captain cannot see his boat
  on the chart. This is non-negotiable.
- [ ] **P1 must work:** if capture drops, every day the computer is
  running is *not* a day of valuable data. This is non-negotiable.
- [ ] **P2 (analyzer) is bonus:** if the analyzer is broken, the
  captain still gets captures. We can iterate on analysis without
  losing data.

If P0 or P1 fails, your first action is `python doctor.py fix --yes`
and verify. If that does not restore it, read
`docs/engineer/13_troubleshooting.md`.

---

## Part 6 — Set up your own working memory (5 minutes)

Before you start any work, leave a breadcrumb for the next agent:

- [ ] **Create today's daily plan:** `docs/PLANS/daily/YYYY-MM-DD.md`
  with the header:
  ```markdown
  # YYYY-MM-DD — Daily Plan

  ## SESSION-NOTE 1 — <your name> — <start time>

  - Goal: <what you're working on>
  - State: <what you verified>
  - Plan: <what you'll do next>
  ```

- [ ] **Commit it:** `git add docs/PLANS/daily/YYYY-MM-DD.md && git commit -m "docs(plans): start daily plan YYYY-MM-DD"`

This is your session handoff. If your session collapses in 30
minutes, the next agent picks up from this commit.

---

## Part 7 — Pick the first task (5 minutes)

Open `docs/PLANS/current.md`. Read the NEXT bullet. That is what to
do first. If `current.md` doesn't exist, follow the 30-day plan in
`05_forward_roadmap.md` week 1.

- [ ] Read `docs/PLANS/current.md`.
- [ ] Identify the next concrete task.
- [ ] If unclear, ask the captain.

---

## The full bootstrap is complete when:

- [ ] All reading done (Part 1).
- [ ] All six system checks pass (Part 2).
- [ ] Layout survey done (Part 3).
- [ ] Recovery tools located (Part 4).
- [ ] P0/P1/P2 priorities internalized (Part 5).
- [ ] Daily plan breadcrumb committed (Part 6).
- [ ] First task identified (Part 7).

**Total time: ~2 hours.**

You are now operational. Welcome aboard.

---

## When you finish this checklist

1. Update `02_state_of_the_system.md` if anything has changed.
2. Commit your daily plan with a SESSION-NOTE describing what you
   learned and what you did.
3. Push to `origin/master`.
4. Tell the captain you're online.

The next agent will read what you wrote and pick up from there.

Ship.
