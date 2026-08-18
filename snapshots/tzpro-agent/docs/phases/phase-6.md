# Phase 6 — DAW Timeline View

> **Status:** Planned — vision-mode
> **Depends on:** Phase 4 (multiple sources flowing), Phase 5 (voice)

## Goal

The dashboard gets a new primary view that looks and feels like a music
DAW (Logic, Ableton, Reaper). Horizontal time axis. Vertical lanes per
source. Zoom in/out. Click a clip → jump to the spatial chart view.

## Layout sketch

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ TZ Pro Agent — DAW View — 2026-07-23 ─────────────────────  [Zoom: 1h ◀▶]   │
├──────────────────────────────────────────────────────────────────────────────┤
│       08:00    09:00    10:00    11:00    12:00    13:00    14:00    15:00 │
│ ─────────────────────────────────────────────────────────────────────────── │
│ Echogram  ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ ▓▓▓▓ (1 per 10min, ~480/day) │
│ AIS       · · · · ▓ · · · · · · · ▓ · · · · · · · · · · · · · · · · · · · · │
│ Voice           ▓▓        ▓▓              ▓▓                                 │
│ Engine    ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │
│ Analysis  ▓▓     ▓▓     ▓▓     ▓▓     ▓▓     ▓▓     ▓▓     ▓▓     ▓▓     ▓▓ │
├──────────────────────────────────────────────────────────────────────────────┤
│ Selected: Echogram @ 11:20 → [Open in spatial view] [Edit] [Delete]         │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Why DAW-style

A fishing day has rhythm. Troll, haul, set, troll, haul. The DAW view
makes that rhythm visible. You see the hauls as gaps, the troll periods
as dense blocks of echogram captures, voice moments as scattered notes.
The captain's eye learns to read the day's shape in one glance.

## What's new in Phase 6

- New dashboard route `/daw`
- Lane configurator (toggle sources on/off, reorder)
- Time-range zoom (1 hour / 1 day / 1 week)
- Clip → jump-to-spatial-view action
- Multi-select for batch operations
- Optional: pattern-detection overlay (e.g. highlight every haul where
  the catch rate exceeded X)

## What unlocks Phase 7

Once the temporal view is first-class, marks-as-output becomes a
spatial complement: the DAW is the timeline, the chart is the space,
marks are how they meet.
