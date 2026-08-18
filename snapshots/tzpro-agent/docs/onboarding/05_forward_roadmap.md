# Forward Roadmap — Where We Are Going

> **Purpose:** Where the system is heading, what the next agent
> should focus on, and what to defer. This complements (does not
> replace) the high-level `ROADMAP.md` in `docs/`. This document is
> opinionated and time-bounded — it reflects what we believe the
> next agent should do, not the maximum we could do.

---

## The three horizons

We think of the work in three horizons:

### Horizon 1: Harden what is built (this month)

The next agent's primary job is to **make what we have unkillable**.

- Capture pipeline running for 30 consecutive days without losing data.
- Doctor 9/9 every morning.
- Tray icon launches daemon automatically.
- Every error path has a clear recovery story.
- Every failure mode is documented in `docs/engineer/13_troubleshooting.md`.

**Definition of done:** The captain can reboot the laptop, leave it
for a week, come back, and find every capture from that week on disk.

### Horizon 2: Wire the analyzer (next 2–3 months)

Once capture is bulletproof, build the analyzer ensemble.

- Build `hermes_ensemble.py` — multi-model parallel caller.
- Add DeepInfra API key to vault (D-009).
- Run first Socratic batch (10 echograms) with `--dry-run` fallback.
- Write feedback to `~/tzpro-personal/echogram-analysis/feedback-batch-1.md`.
- Iterate: tier-3 to tier-4 when rolling agreement ≥ 0.7 over 50 missions.

**Definition of done:** The captain asks "what did we see at the
southwest corner yesterday?" and the chatbot answers with specific
echogram moments, not a generic response.

### Horizon 3: Federation (Phase 8, 6+ months out)

Once the local app is so good the captain uses it every day, build
cloud federation.

- SQLite → D1 sync.
- Files → R2 sync.
- Vectors → Vectorize sync.
- Multi-device dashboard.
- Multi-boat fleet view (only for boats whose captain opts in).

**Definition of done:** Two boats running tzpro-agent can compare
their echogram libraries without sharing raw GPS data.

---

## The next 30 days (concrete tasks)

These are the tasks the next agent should pick up **in order**. Do not
skip ahead. Do not parallelize without reading D-007 first.

### Week 1 — Observability hardening

1. **Add timestamped boot-log to every daemon.** Right now, if the
   capture daemon crashes at 03:00, we don't know unless the captain
   notices the tray icon is gray. Add a log line every time the
   daemon starts, every time it detects TZ Pro, every time a capture
   succeeds or fails.
2. **Add a daily `doctor.py check` cron-style entry** that writes to
   `logs/doctor_<date>.log`. If we go 24 hours without a successful
   doctor run, something is wrong.
3. **Add a "last seen alive" heartbeat** to the stampfile that the
   capture daemon writes every 60 seconds. If the heartbeat goes
   silent, the tray should turn red.

### Week 2 — Recovery hardening

4. **Make `fix_priority0.bat` smarter.** Right now it kills and
   restarts. Make it detect "already running" and skip the kill if
   so. Make it print a clear summary at the end ("Bridge: OK,
   Dashboard: OK, TZ Pro: NOT CONNECTED — open TZ Pro and connect
   to localhost:6006").
5. **Add a one-click "I forgot to start the tray" shortcut** that
   runs `fix_priority0.bat` and then launches the tray. Captain
   should not need to think about order.
6. **Test the recovery story.** Reboot the laptop. Time how long it
   takes to get back to a healthy state. Target: under 60 seconds
   if TZ Pro is already running.

### Week 3 — Data durability hardening

7. **Add a daily backup script** that copies the captures folder
   to a second drive (or OneDrive). Verify the backup runs.
8. **Add a "verify yesterday's captures" task** to the cascade
   daily loop (when it gets wired). Catches silent data corruption.
9. **Add a `--strict` flag to `verify`** that fails if any capture
   in the last 24 hours has missing JSON or MD sidecar.

### Week 4 — Documentation pass

10. **Read every doc in `docs/engineer/` and verify the file:line
    citations.** If a citation is wrong, fix the doc. If a claim is
    no longer true, fix the doc.
11. **Update `docs/PLANS/daily/2026-MM-DD.md`** with the SESSION-NOTE
    for every session this month. The captain will read these.
12. **If anything in `02_state_of_the_system.md` has changed, update
    it.** This document is the ground-truth handoff.

---

## The next 90 days (analyzer wiring)

### Block 1 — Build the ensemble

1. **Read `~/hermes-nerve-center/`** — understand the existing harness.
2. **Read `providers/base.py`** — understand the provider abstraction.
3. **Read `providers/deepinfra.py`** — confirm DeepInfra provider works.
4. **Build `hermes_ensemble.py`:**
   - Accept a list of moments.
   - For each moment, call all enabled models in parallel via
     `asyncio.gather`.
   - Score agreement (cosine similarity of embeddings, or simple
     yes/no vote for classification tasks).
   - Return consensus verdict + per-model responses.
   - Support `--dry-run` mode that fakes model responses with
     timestamped stubs.
5. **Wire `hermes_worker.py`** to call ensemble.
6. **Test with 10 hand-picked echograms** — verify the harness works
   end-to-end with `--dry-run`.
7. **Get the DeepInfra API key from the captain** and store it in
   the vault.
8. **Run the first real Socratic batch.**

### Block 2 — Add the cascade

9. **Wire `cascade/decaminute.py` to run the analyzer on each new
   capture.** This is the first end-to-end "capture → analysis"
   loop.
10. **Add `cascade:daemon` to the doctor check.** Verify it is alive.
11. **Add cascade logs to `logs/`.** Make failures loud.

### Block 3 — Surface the analysis

12. **Update the dashboard** to show the latest analysis alongside
    the capture.
13. **Update the tray** to show "Latest analysis: <verdict>" in the
    status tooltip.
14. **Ship a "yesterday's school behavior" daily summary** that the
    captain reads at the morning brief.

---

## What to defer

These are real ideas. They are not what the next agent should focus
on. Defer them until Horizon 2 is done.

- **Voice STT/TTS (Phase 5):** the captain has not asked for this
  yet. Don't pre-build.
- **Multi-session chat (Phase 3):** only one captain on one boat for
  now. Defer until Phase 8 (cloud) when multi-user makes sense.
- **DAW timeline view (Phase 6):** the dashboard's center panel is
  fine for now. Defer until the data model proves out.
- **Marks-as-output (Phase 7):** writing TZ Pro marks from the model
  is a write-back integration that requires careful TZ Pro API
  knowledge. Defer.
- **Vector-DB spatial/temporal (Phase 9):** we need more captures
  before vector search is useful. Defer.
- **Projection layer (Phase 10):** the dashboard works. The
  projection layer is the IDE-shape of the future. Defer until we
  have a use case the dashboard cannot serve.

---

## The "should I build this?" decision tree

When the next agent proposes a feature, walk this tree:

1. **Has the captain asked for it?**
   - No → Defer. Build it when asked.
   - Yes → Continue.

2. **Does it require data we don't have yet?**
   - Yes → Defer until we have the data. Capture is foundation.
   - No → Continue.

3. **Does it make the capture pipeline more durable?**
   - Yes → Build now. Capture is the killer feature.
   - No → Continue.

4. **Does it block another in-flight feature?**
   - Yes → Build now.
   - No → Continue.

5. **Does it require more than a week of focused work?**
   - Yes → Break it into smaller pieces. Each piece ships.
   - No → Continue.

6. **Will the captain use it every day?**
   - Yes → Build now. Daily-use features compound.
   - No → Continue.

7. **Will the captain use it weekly?**
   - Yes → Build when we have a free week.
   - No → Continue.

8. **Will the captain use it monthly?**
   - Yes → Build when we have a free month.
   - No → Continue.

9. **Will the captain use it yearly?**
   - Yes → Document the idea in `docs/research/` and revisit next year.
   - No → Don't build it.

---

## The "this is broken, fix it" decision tree

When something breaks (capture stops, GPS drops, tray crashes):

1. **Is TZ Pro running?** If not, that's the bug — the captain
   closed TZ Pro. Ask if they want it back up.
2. **Is the bridge running?** `Get-Process python | Where-Object
   {$_.MainWindowTitle -like "*nmea*"}`. If not, run `fix_priority0.bat`.
3. **Is the daemon running?** Check `capture_daemon.stamp.json`. If
   not, run `python capture_daemon.py run --auto`.
4. **Are there errors in `logs/`?** Read them. Fix the underlying cause.
5. **Is COM6 still plugged in?** Check Device Manager. Sometimes the
   USB cable comes loose.
6. **Is the GPS antenna still connected?** u-blox needs power and sky.
7. **Did the captain reboot the laptop?** Run the fix script.
8. **Has anything changed in the environment?** New TZ Pro version?
   New Windows update? New USB driver?

If none of the above solves it, capture diagnostic output and ask the
captain for help. Do not improvise fixes you don't understand.

---

## Success metrics for the next 30 days

- 30 consecutive days with no silent data loss.
- Doctor 9/9 every morning for 30 days.
- Tray icon visible in the wheelhouse every time the captain looks.
- Capture cadence stable at 10 minutes.
- GPS fix quality consistently ≥ 1, satellites ≥ 4.
- Zero unhandled exceptions in `logs/`.
- One captain-reported issue resolved end-to-end within 24 hours.

If all of these are true at day 30, Horizon 1 is done. Move to
Horizon 2.

---

## The one thing to remember

The system is **already valuable**. The capture pipeline is
**already durable**. The captain is **already using it**. The next
agent's job is not to add features. The next agent's job is to
**make sure the captain never has to think about whether the system
is working**.

When the captain stops thinking about the system, we have won.
