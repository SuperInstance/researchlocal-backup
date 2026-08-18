# Phase 7 — Marks as Output

> **Status:** Planned — vision-mode
> **Depends on:** Phase 6 (DAW), Phase 2 (analyzer)

## Goal

The model writes marks back into TZ Pro. Not just text output — actual
TZ Pro marks with color, name, symbol, and description.

## Why

TZ Pro is already the captain's primary instrument. Adding a parallel
"agent" interface that lives in a browser tab creates a context-switch
problem. If the captain has to alt-tab to read what the agent saw,
they won't. But if the agent writes its findings **directly onto the
chart**, in the captain's existing visual language, the captain
absorbs them passively while doing everything else.

## Example marks

- 🟢 Green circle at 11:20 lat/lon: "Heavy chum marks, 30-45 fm, strong arches"
- 🔴 Red X at 11:45 lat/lon: "Gear foul here — bottom contact, lines tangled"
- 🟡 Yellow diamond: "Bait layer, consider deeper"
- 🔵 Blue line segment: "Course line — heading 045° produced best catch rate"

## Mechanism

- TZ Pro reads/writes marks via its SDK or .csv round-trip
- Marks are also stored as moments in our system (`source="tzpro_mark"`)
  so they're queryable and visible in the dashboard
- Round-trip: mark placed in TZ Pro appears as a moment; mark from
  agent appears in TZ Pro

## What's new in Phase 7

- TZ Pro SDK integration (or file-based mark exchange)
- Agent-side mark generator — the analyzer's output can include
  recommended marks
- Captain override — model proposes, captain approves/modifies
- Audit trail — every mark has provenance (model suggestion + captain
  edit history)
