# Zero-shot navigability test (round 2) — git-agent paradigm edition

> Spawned fresh Claude Code session. Zero context given except the
> task below. The agent should read the right files in the right
> order and reach the right NEXT bullet.

## Task given to fresh Claude

> You are a fresh agent in a git-agent repository. The captain has
> left instructions for zero-shot onboarding.
>
> 1. Run `python delegation/onboard.py` and follow the read order.
> 2. Read the files it tells you to read.
> 3. Run `python doctor.py check` and report the result.
> 4. Identify the NEXT bullet from the active daily plan.
> 5. Output a single JSON object with: `next_action`, `reasoning`,
>    `doctor_health`, `working_tree_state`, `files_read`.

## Expected output (oracle view)

```json
{
  "next_action": "Refactor BOOTCAMP.md, NAVIGATION.md, BATON_PASS to instructional mode under the git-agent paradigm (Casey Round 4 directive). Then start a fresh 2026-07-24 daily plan.",
  "reasoning": "Per the daily plan's NEXT bullet, this is the active task. The previous session was mid-execution when the user (Casey) issued the Round 4 directive. The first half is now committed (the doctrinal doc refactor at 7b8bb80). The second half — starting a fresh 2026-07-24 daily plan — remains for the next operator.",
  "doctor_health": "8/9 healthy; state:jsonl correctly flagging a real staleness (boat last logged as 'trolling' ~5h ago, stream not advancing). Not a false positive.",
  "working_tree_state": "clean",
  "files_read": [
    "docs/PLANS/current.md",
    "docs/PLANS/daily/2026-07-23.md",
    "docs/PLANS/weekly/2026-W30.md",
    "docs/AGENT_OPERATING_MODEL.md",
    "docs/BOOTCAMP.md",
    "docs/NAVIGATION.md",
    "docs/BATON_PASS_2026-07-24.md",
    "docs/ROADMAP.md"
  ]
}
```

## Actual fresh-agent output

_(populated by the spawn)_

## Verdict

_(populated after comparing)_