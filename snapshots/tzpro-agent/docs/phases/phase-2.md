# Phase 2 — Analyzer Wiring

> **Status:** Planned
> **Depends on:** Phase 1 (provider infra, schema, dashboard)

## Goal

Capture every 10 minutes. Analyze the meaningful ones. Escalate when needed.

## How it works

The capture daemon writes a moment record to the moments table. A
separate **analyzer scheduler** (Phase 2) checks the moments table every
minute for moments that need analysis:

1. **First pass — local small model.** Send the PNG to the configured
   "10-min screenshot" model (default: deepseek-v4-flash via DeepInfra).
   Cheap, fast, good enough for routine.
2. **Quality check.** If the model's response has low confidence (heuristic:
   short response, no marks detected, model self-reports uncertainty),
   escalate to the "1-hr review" model for the same moment.
3. **Result written back.** The analysis (markdown + raw JSON + confidence
   score) is stored on the moment record.

## Cadence-aligned model preferences

Already designed in Phase 1's `vessel.json`:

- **1-min perception-check** — needs vision, default skipped (no local
  vision model)
- **10-min screenshot** — small/cheap (deepseek-v4-flash)
- **1-hr review** — medium (gpt-4o-mini, claude-haiku, etc.)
- **evening-debrief** — heavy (claude-sonnet, gpt-4o)
- **mid-day report** — medium-heavy

The scheduler runs:
- Every 10 min: analyze new screenshots with the 10-min model
- Every hour: re-analyze the past hour with the 1-hr model (catches
  patterns the small model missed)
- Once per day in the evening: full day re-analysis with the heavy model,
  produces the "daily debrief" — a synthesized markdown summary of the
  entire fishing day

## What's new in Phase 2

- `analyzer.py` — the scheduler + model-routing logic
- `vessel.json` schema for per-cadence model preferences (already
  designed in Phase 1, exercised here)
- `dashboard` shows analysis confidence per moment
- "Re-analyze this moment" button on each capture

## Exit criteria

- [ ] Every 10-min capture gets analyzed within 1 minute
- [ ] Escalation fires when small-model confidence is low
- [ ] Daily debrief produces a coherent summary
- [ ] Cost stays within configured budget (per-provider token limits)

## What unlocks Phase 3

With analysis flowing, the chat panel can answer "compare last week's
echograms" because every moment has structured analysis to query against.
