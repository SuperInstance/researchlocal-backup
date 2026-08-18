# Baton Pass — Session 47/48 Collapse → Resumed Self

*Written by the recovered session after the context-window blowout on
2026-07-23, when the previous agent ran past 1.4M local tokens and the
API rejected every call with `invalid_request_error (2013) context
window exceeds limit`. Casey captured the terminal scrollback in
`Documents/minimaxhistorybackup.md` and `/clear`-ed the session. This
doc is the bootcamp for the resumed agent.*

---

## Recognition phrase (for whoever picks this up)

*"This is the resumed agent picking up after the 80k collapse.
Phase 1 was almost complete but uncommitted. The ship metaphor
(monologue agent) was designed but never scaffolded. Cloudflare
wrangler is installed but unauth'd."*

---

## What the previous session was doing (compressed)

We were building a **troller fishing agent system for F/V Eileen**, and
the previous session landed on:

1. A working **`tzpro-agent/` Phase 1 build`** — bridge, dashboard,
   tray, capture_daemon, doctor with 9 checks, vault, vessel config.
   This is on disk. It runs. It's **uncommitted** in git.
2. A **ship-metaphor design for `tzpro-monologue/`** — internal
   monologue agent where the repo IS the agent (Theseus/Eileen), local
   model (granite4.1:8b) is the always-on self-data-miner that builds
   its own power armor, cloud (mother agent) is developmental
   environment not oracle. **Pure design — no code on disk.**
3. A **Cloudflare wrangler 4.72.0 install** salvaged into
   `tzpro-cloudflare/` but **never authenticated** to Casey's account.
4. A **Claude Code subagent delegation harness**
   (`delegation/claude_runner.py`) — works, only one task dispatched
   before collapse.

Casey's last directive, right before context blew:

> "yes keep going as far as you can. also, be high quality. you have
> claude code installed with glm-5.2 acting as opus 4.8. use claude a
> lot to offload work. be the orchestrator and both you can claude
> can do subagents to further structure delegation"

So the resumed agent should: **commit the uncommitted work first**,
then **be an orchestrator**, then **build monologue as the next
subsystem**, with Claude Code as the heavy-lifter.

---

## What the resumed agent should internalize (the bootcamp)

### The user's core philosophy (NEVER FORGET)

- **Models are iterators of context, not oracles of knowledge.**
- **The repo IS the agent.** Like F/V Eileen — built 1935, replanked,
  reworked; *Eileen* persists while the wood changes.
- **Internal monologue = always-on self-data-miner.** Runs whenever
  the computer has spare cycles. Re-reads transcripts of human, self,
  peers; finds patterns that cost tokens/iterations; writes back to
  the hull.
- **Workspace as a ship.** Hull (persisted self), slip (where new
  construction happens), chandlery (tool library), log (captain's log
  = boat's log, same thing).
- **Local model = bright middle-schooler** building its own power
  armor of tools/libraries. Doesn't need to be smart; needs to be
  *energetic.*
- **Mother agent (cloud) = developmental environment, not oracle.**
  Like a kid overhearing parents narrate before they have words.
  Boat-agent absorbs. Then diverges.
- **LoRA flywheel:** cloud high-quality outputs → training data →
  local gets better → less need for cloud.
- **Cost gaming:** time-aware throttle on free tier. Accelerates when
  under cap, decelerates near reset. Per-hour budgets that can be
  vibe-coded.
- **Domains held:** `fisherman.systems`, `oceanready.systems`,
  `pincher.systems`, `pincher.win`. Don't over-claim.

### Patterns the previous agent used well (DO THESE)

- **"Say it back in three layers"** before building. Domain → Repo-
  as-agent (Theseus) → Mother-agent. Casey thinks in analogies; the
  analogies are the design.
- **Ask 2-3 clarifying questions max, then build.** Casey wants the
  build, not the conversation.
- **Smallest useful slice, then expand.** The internal monologue
  loop, the cost throttle, the LoRA pipeline — all can be sliced.
- **E2E smoke tests after each subsystem.** `scripts/_test_*.py` is
  the pattern.
- **ASCII-only in `.ps1` and `.bat`** — em-dash (0x97) mangles under
  Windows codepage.
- **`pystray` reactivity:** rebuild `Menu` object, assign
  `icon.menu = new_menu`, call `update_menu()`. Not the factory
  pattern.
- **DPAPI + AES-GCM:** never pass protected blob to KDF; only the
  32 raw bytes. Asymmetry = silent corruption.

### Patterns that caused the collapse (DO NOT REPEAT)

- **No commit happened until context collapsed.** All Phase 1 work
  was sitting in working tree when the API rejected. **NEVER let
  working tree carry >30 min of uncommitted work.**
- **The terminal log itself became an 8000-line artifact** that
  nearly broke the resumed session just to read it. **Minimize
  stdout/stderr in tools. Pipe to `Out-Null` aggressively.** The
  terminal scrollback is the bottleneck, not the disk.
- **40+ steps on a desktop shortcut.** When OneDrive Desktop
  detection got tricky, should have asked Casey to double-click the
  bat manually and moved on.
- **Test scripts left as dead weight** in `scripts/_test_*.py`. Move
  completed tests to `tests/` or delete; the `scripts/` dir is for
  ops scripts.
- **Did not write ONBOARDING.md / bootcamp proactively** for the
  next session. This doc is the corrective action.

### The collapse itself taught us

- Context wall: API reported 80k, locally we tracked 1.4M tokens
  before the 400-error. The wall is hard.
- Summarization can recover *some* history but **not file contents,
  not exact edit text**. The ONLY things that survive intact are:
  files written to disk, git commits, terminal logs the user
  preserved.
- Reading our own log to resume cost a LOT of tokens. Future
  sessions should write a `BATON_PASS_*.md` proactively rather
  than relying on scrollback.

---

## Concrete state at handoff (2026-07-23)

### tzpro-agent/ — what's working and uncommitted

```
.gitignore                 M   (touched)
cascade/daemon.py          M
cascade/daily_loop.py      M
cascade/decaminute_loop.py M
cascade/hourly_loop.py     M
cascade/retention.py       M
doctor.py                  M   (extended with dashboard + capture_daemon checks)

??  agent_router.py
??  capture_daemon.py
??  dashboard.py
??  dashboard/
??  delegation/            (claude_runner.py + 1 task file)
??  providers/
??  roster/
??  schema/
??  assets/                (icon-shortcut-256.png etc)
??  scripts/_make_shortcut.ps1   (encoding+OneDrive path fixed)
??  scripts/_stop_tray.bat
??  scripts/_test_capture_daemon.py
??  scripts/_test_phase1_e2e.py
??  scripts/_test_providers.py
??  scripts/_test_router.py
??  scripts/_test_tray_app.py
```

Git log is clean through `89593e6 feat(vault+doctor+roadmap)`. The
above is the uncommitted Phase 1 work.

### Components verified alive (per last doctor check)

- bridge :6006 — PID 11940, 8 sats, fix_q=1
- ollama — all 4 models loaded
- vault — DPAPI + AES-GCM round-trip OK
- dashboard :8090 — `/api/chat` returns real granite content
- doctor — 8/9 healthy (state:jsonl flagged because boat stationary,
  expected)
- capture_daemon — process model verified, lifecycle works
- tray_app — menu builds, 9 actions
- start_capture_tray.bat — links to pythonw.exe

### What's broken / unfinished

1. **Desktop shortcut to OneDrive Desktop** — script fixed but
   not yet verified end-to-end after the OneDrive path branch.
2. **Commit + push** — see git status above.
3. **`state:jsonl` doctor check** — should accept DOCKED state as
   valid (don't fail when stationary).
4. **Cloudflare auth** — wrangler 4.72.0 in `tzpro-cloudflare/`,
   needs API token or `wrangler login`.
5. **Claude Code delegation harness** — built, smoke-tested, but
   only one task dispatched. Use it more.

### What's not yet on disk at all

1. **`tzpro-monologue/`** — the ship-metaphor internal monologue
   agent. Hull/, slip/, chandlery/, monologue.py. Pure design last
   session; **no scaffolding written.**
2. **Cost gaming throttle** — time-aware budget tracker that
   accelerates/decelerates per CF reset window. Casey specifically
   called this out.
3. **LoRA distillation pipeline** — cloud-output → training data →
   local-model improvement. Out of scope for current phase but
   architectural hooks should land now.

---

## Recommended resumption order (smallest first)

1. **Commit Phase 1** with `feat(phase-1): capture_daemon + tray_app
   + dashboard + doctor extensions + delegation harness` — this
   alone is several hours of work. Commit before touching anything
   else.
2. **Verify shortcut** by launching `_make_shortcut.ps1`, confirm
   `.lnk` exists on OneDrive Desktop, verify `TargetPath`.
3. **Tune `state:jsonl`** to not fail in DOCKED state.
4. **Push to remote** if network cooperates.
5. **Scaffold `tzpro-monologue/hull/keel/AGENT.md`** (the
   constitution) and the ship directory tree. Match the design
   Casey and previous-me converged on.
6. **Build cost throttle** as a small standalone module —
   `cost/throttle.py` + tests. Casey wants this and it's measurable.
7. **Dispatch Claude Code** on Phase 2 code-review tasks while we
   build monologue scaffolding.
8. **Cloudflare auth** when network cooperates (offer API-token path
   first; it's headless-friendly).

---

## One-line summary for Casey

"Resumed agent is here. Phase 1 is built but uncommitted — first
move is to commit it. Ship metaphor and cost gaming are next on
deck. I'm going to be more disciplined about commits and terminal
chatter this round."