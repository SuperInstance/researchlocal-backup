# Phase 3 — Multi-Session Chat

> **Status:** Planned
> **Depends on:** Phase 1 (sessions table), Phase 2 (analysis flowing)

## Goal

Each person on the boat gets their own private view of the same underlying
moments. Captain sees one thing, crew sees another, but they're all reading
from the same SQLite database on the ProArt.

## How it works

- Each browser tab = one session (already true from Phase 1)
- New Phase 3 addition: **named sessions** + **invite links**
- Session names: "Casey — wheelhouse", "Sam — slush-ice station",
  "Observer — cabin"
- Invite link: `http://proart.local:8090/join/{token}` — token is a
  pre-shared session ID that lands in someone's browser

## Privacy model

- All users see the same underlying data (moments, analyses)
- Chat history is per-session (private)
- Preferences are per-session
- No user accounts yet — the LAN is the trust boundary

## What's new in Phase 3

- Session naming UI
- Invite-link generation (captain can mint N links with TTL)
- Session list / switcher in the dashboard header

## Exit criteria

- [ ] Captain can mint invite links
- [ ] Crew member joins from phone, sees their own chat history
- [ ] Crew member cannot see captain's chat history (and vice versa)

## What unlocks Phase 4

With session management mature, we can add **per-source permissions**
later (e.g. engine gauges visible only to engineer sessions).
