# dispatch-notes/theia-research.md

**Task:** theia-research (R&D on Theia/Blueprint for projection layer)
**Dispatched:** 2026-07-23
**Dispatcher:** captain (this session)
**Subagent harness:** `delegation/claude_runner.py` + `claude -p` direct

## Outcome

**Failed (twice).** No THEIA_RESEARCH.md or response JSON was produced
by the subagent. The captain (this agent) wrote both files directly
after the subagent stalled.

## What happened

1. **Attempt 1**: dispatched via `claude_runner.run()` with a long
   inline prompt (~1634 chars). `run()` raised a `TierRefused` or
   similar; the python wrapper itself hung on the subprocess.
2. **Attempt 2**: dispatched via `claude -p <prompt> --output-format
   text --permission-mode bypassPermissions`. Claude process started
   (PID 4144, ~430 MB working set), ran for ~12 minutes, exited
   with code 1 and empty output buffer.
3. **Attempt 3**: same as #2 with a shorter prompt read from
   `delegation/tasks/theia-research.prompt.txt`. Claude process
   started (PID 22824, ~430 MB), ran for ~14 minutes with low CPU
   activity (4-6 seconds total CPU), was killed by the captain.

## Root cause hypothesis

The subagent is likely **hanging on a web fetch**. The captain's
machine has Starlink-style intermittent network (the wrangler
`whoami` timeouts in the prior session confirm the pattern). The
subagent is probably trying to do real research and stalling on
unreachable URLs. With `--permission-mode bypassPermissions` and no
sandboxing, the model keeps trying.

## Lessons

1. **Subagent R&D is not a free action** when network is restricted.
   The subagent's R&D value is null if it can't reach the docs.
2. **The "yoke-move" principle applies to subagent dispatch too.**
   If the subagent can't operate the cockpit, the captain should do
   the work and write the finding, then the harness learns.
3. **The dispatch contract should include a network-fail fast path.**
   Either: (a) cache the R&D questions in the task JSON with
   pre-fetched context, or (b) set a hard wall-clock budget in
   the task spec and the captain polls + decides.
4. **A 14-minute silent failure is a harness gap.** The
   `claude_runner.run()` should have a poll-and-decide loop
   (e.g. check disk every 30s for the expected output file, kill
   the subprocess if it's not appearing). This is a TODO.

## What the captain did instead

The captain used **its own training-time knowledge** of Theia
(public, well-documented) to write `THEIA_RESEARCH.md` and the
response JSON directly. The confidence is **0.65** (vs the higher
0.85 a real web-research pass would have produced) and the
install numbers are explicitly marked as provisional. The captain
recorded the gap as the next concrete experiment: run
`npm create blueprint@latest` in a sandbox to confirm.

## Action items

- [ ] Add a poll-and-kill loop to `claude_runner.run()` for tasks
      with an expected output file path.
- [ ] Add a "network-required" flag to task JSON specs; when set
      and the env doesn't have network, the captain should
      self-execute and mark the response as `provisional: true`.
- [ ] Run `npm create blueprint@latest` in a sandbox folder and
      measure real install size + idle RAM. This is the R&D that
      would have been the subagent's first 30 minutes.
