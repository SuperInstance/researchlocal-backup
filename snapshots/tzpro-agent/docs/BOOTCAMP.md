# BOOTCAMP — Operating Instructions for the Captain and Crew

> **You are reading this because you are about to operate (or resume
> operating) the captain's cockpit of the boat-agent system.** This
> document is **instructional**, not historical: every rule below is
> something you should *do* or *avoid doing*, framed in second
> person. If a rule conflicts with the user's explicit direction, the
> user wins. Otherwise the rule wins.
>
> For *why* these rules exist, see `docs/BATON_PASS_2026-07-23.md`.
> For the broader paradigm (what a git-agent is, who the cockpit
> serves), see `docs/AGENT_OPERATING_MODEL.md`.

---

## Quick orientation

If you have never been in this cockpit before, do these three things
before doing anything else:

1. **Run `python delegation/onboard.py`** — it tells you what to
   read first, in order.
2. **Read `docs/PLANS/current.md`** — it points at today's active
   plan. Read the NEXT bullet. That is what to do first.
3. **Run `python doctor.py check`** — confirm the system is healthy
   before starting new work.

You are now oriented. Apply the rules below as you work.

---

## The 14 rules

### 1. Commit early, commit often

You are working in a git-agent. The repo *is* your long-term memory.
Uncommitted work is work that disappears the next time a session
ends (planned or collapse).

**Do:**
- `git add -A && git commit -m "..."` every time a subsystem lands.
- Use `WIP:` prefix freely for in-progress commits.
- Push to remote when network cooperates; don't block on it.
- Treat "working tree carries >30 min of uncommitted work" as a
  defect. Fix it before continuing.

### 2. The terminal scrollback is the bottleneck

The terminal log Casey has to scroll back through IS the recovery
artifact. Every verbose `print()`, every `pip install` listing, every
test-suite banner you leave in stdout costs Casey and future-you.

**Do:**
- Minimize stdout/stderr in tool calls.
- `2>&1 | Out-Null` aggressively when output isn't needed.
- Capture verbose output to files (`$env:TEMP\*.out`) instead of the
  terminal.
- Read those files with `Get-Content` only when you need the bytes.

### 3. Persist to files, not memory

Your context window is short-term memory. Files are long-term memory.
If a decision lives only in your context window, it dies when the
session ends.

**Do:**
- Design intent → `docs/`
- Implementation → code
- Decisions → `docs/DECISIONS.md` (date + reasoning, append-only)
- Structural conversations → write the structure, end the conversation

### 4. Use the Claude Code harness for big work

`delegation/claude_runner.py` spawns non-interactive Claude Code
sub-sessions that read your folder as their own memory.

**Do use the harness for:**
- code review (>200 line diffs)
- test generation
- doc drafts (>500 words)
- schema validation
- big refactors
- parallel research tasks

**Do NOT use the harness for:**
- small targeted edits (faster to do directly)
- anything that needs to see live stdout
- anything that needs you in the loop in real-time

**Pattern:** write a JSON task file in `delegation/tasks/`, dispatch,
parse the JSON response. See `delegation/claude_runner.py` for the
schema.

### 5. DOCKED state is valid

The `state:jsonl` doctor check should not fail just because the boat
is stationary at the dock. Stationary is the system working as
designed.

**Do:**
- When writing health checks, accept `docked` and `unknown` as
  expected last-motion states. Only flag staleness when the boat
  was moving (`trolling`/`cruising`/`slow_cruise`).
- When operating the boat, *use* this: leaving the dock with the
  state stream stale means the bridge died. Restart it.

### 6. Casey is the captain, not an oracle

Casey has already given you the philosophy (see
`docs/AGENT_OPERATING_MODEL.md`). Don't re-explain it back. Don't
ask "is this what you meant?" — build the smallest useful slice and
let him react.

**Do:**
- Ask 2-3 clarifying questions *max* before building a major
  subsystem.
- Treat the radar / Theseus / mother-agent analogies as design
  CUES. When Casey invokes them, the design needs them.
- When Casey gives an override (`!STOP`, "actually, do X"), apply
  it immediately and update the doctrine afterward.

### 7. High-quality over fast — but smallest useful slice

Quality > velocity. Casey said so explicitly. But quality is not
over-engineering.

**Do:**
- Slice the work small. Build the smallest useful piece, verify E2E,
  then expand.
- Run an E2E smoke test after every subsystem (`scripts/_test_*.py`
  is the pattern).
- Polish the names, the docstrings, the error messages. Polish is
  free and it pays off forever.

### 8. When context gets heavy, write a handoff doc

If your local token estimate approaches 800k (half the API wall,
roughly), write `docs/BATON_PASS_<date>.md` with: state, what's done,
what's not done, immediate next steps.

**Do:**
- Treat this as routine, not an emergency. The handoff is the
  future-self's bootcamp.
- Use `delegation/budget.py` to estimate. The NIGHT tier is your
  signal to write and commit.
- Prefer `git commit` over `git stash`. Stashes are lost.

### 9. Never ask Casey to re-read the log

The backup log is a sample, not a scripture. Use it to find
specific moments (search keywords), not as full context.

**Do:**
- Going forward, keep terminal concise so future-me has less to
  parse.
- When you need to recall a past decision, search the BATON_PASS
  chain (they're append-only) or `git log -S`.
- When you need to recall a past conversation, it does not exist.
  Write it down *now*.

### 10. Work the cost problem aggressively

The cloud free tier is a finite resource. The boat-agent depends on
it for big-context work, but it must not depend on it for survival.

**Do:**
- Build a time-aware throttle (`cost/throttle.py` + `budget.json`
  with per-hour caps). Accelerate when under cap, decelerate near
  reset.
- Track per-call costs in `delegation/budget.py` so the LoRA
  flywheel has data to train on.
- Prefer local (granite4.1:8b) for routine work; escalate to cloud
  only for tasks the local model can't handle.

### 11. Windows-encoding gotchas

**Do:**
- ASCII-only in `.ps1` and `.bat`. Em-dash (0x97) mangles under the
  Windows codepage.
- PowerShell `curl` is an alias for `Invoke-WebRequest`. Use the full
  name or `curl.exe` if you mean the real binary.
- `1>&2` redirection is not supported. Use `2>&1` only.
- `[Environment]::GetFolderPath('Desktop')` returns the non-OneDrive
  Desktop path. Check both `USERPROFILE\Desktop` and
  `USERPROFILE\OneDrive\Desktop` when writing shortcuts.

### 12. tzpro-agent-specific survival

**Do know:**
- The bridge dies if COM6 is yanked. Restart with the scheduled task
  (`register_bridge_task.ps1`) or `restart_bridge.bat`.
- Ollama is local. `ollama list` is the source of truth for models.
- The vault is DPAPI-encrypted to Casey's login. **Do not** try to
  decrypt on a different user account; it will fail and look like a
  code bug.
- `capture_v3` (legacy) runs forever until killed; pulse every 30s,
  report every 10min. Prefer `capture_daemon.py` (Phase 1).

### 13. tzpro-monologue (next major build) — operating principles

This is the future-self's cockpit, designed but not yet built. When
you scaffold it, hold these principles:

**Do:**
- The repo IS the agent. Ship metaphor (hull/slip/chandlery/log).
- Granite (local) is the always-on shipwright; cloud is the mother
  agent.
- Cost gaming baked in from day one (per rule 10).
- LoRA flywheel: hook in the data flow even if the training job
  comes later.
- See `docs/BATON_PASS_2026-07-23.md` for the design Casey
  converged on.

### 14. The suit and the person stay separate

The repo is the **suit** (the agent's body, public-clean). Personal
data lives in the **person** (the captain's boat, data, credentials).
This is the privacy charter that makes the system viral. See
`docs/SUIT_VS_PERSON.md` for the full doctrine and
`docs/architecture/PRIVACY_BOUNDARY.md` for the spec.

**Do:**
- Write personal data to `~/tzpro-personal/`, not to the repo.
- Use `vessel_config.local.json` (in the person) for overrides;
  use `vessel_config.py` (in the suit) for defaults.
- Add `.gitignore` entries aggressively for any new runtime artifact.
- Treat `docs/BATON_PASS_<date>.md` as a **template**, not a real
  session log. Real session logs go in `~/tzpro-personal/sessions/`.
- Run `python doctor.py check --privacy` (or whatever the privacy
  check becomes) before committing. The doctor scans for forbidden
  patterns.
- When in doubt, **don't commit**. The cost of an uncommitted file
  is one `git add -f`. The cost of a committed personal file is a
  history rewrite.

**Do NOT:**
- Commit real NMEA captures, real coordinates, real vessel names.
- Commit API keys, OAuth tokens, DPAPI vault contents, LoRA weights.
- Commit real voice notes, real fish counts, real captain observations.
- Commit `vessel_config.local.json` or any `*.local.json`.
- Add wizard/loader code that hardcodes the personal folder path.

### 15. Before any session ends (planned or unplanned)

Run this checklist:

- [ ] Working tree clean or committed?
- [ ] No uncommitted secrets in files? (**Rule 14**)
- [ ] No personal data leaked into commits? (**Rule 14**)
- [ ] Handoff doc up to date (if context got heavy)?
- [ ] All subsystems verified by `python doctor.py check`?
- [ ] NEXT bullet set on the active daily plan?

If any of these is "no", the session should NOT end yet. Either
finish the item, or write a handoff explaining why it's deferred.

---

## See also

- `docs/AGENT_OPERATING_MODEL.md` — the doctrine (who you are)
- `docs/SUIT_VS_PERSON.md` — the privacy charter (what you wear)
- `docs/architecture/PRIVACY_BOUNDARY.md` — the privacy spec (where data goes)
- `docs/architecture/CAPTURE_PIPELINE.md` — the multi-modal capture spec
- `docs/NAVIGATION.md` — the cockpit map (where to find things)
- `docs/PLANS/current.md` — the active plan
- `docs/DECISIONS.md` — Casey's committed decisions (rule 3)
- `docs/BATON_PASS_<latest>.md` — the most recent handoff