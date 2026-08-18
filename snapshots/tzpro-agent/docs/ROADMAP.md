# TZ Pro Agent — Flagship Roadmap

> The living vision for the boat-agent platform.
> This document is the user-facing north star. Each phase below has its own
> detail file in [`phases/`](phases/). Only the current phase is in focus
> during a build; later phases are aspirational and may pivot as discoveries
> reshape them.

---

## What we're building

A **local-first, capture-and-analysis system** for the modern fishing boat.
At its core it watches a TZ Pro / Nobeltec sounder screen, captures frames
on a steady cadence, indexes them by time and location, and serves them
back through a web dashboard that anyone on the boat's LAN can use.

But the system isn't really about sounder pictures. It's about **moments**.

A moment is anything that happens at a specific time and place on the boat:

- A sounder pulse (every 30s while running)
- A formal screenshot (every 10 minutes on the hour boundary)
- A human voice note ("24 chum on the lines, 3 pinks near the top")
- An engine gauge reading (RPM, oil pressure)
- An AIS contact (vessel name, range, bearing)
- A model-generated analysis ("bottom marks suggest feeding school")
- A TZ Pro mark placed by the captain

**Every moment has the same shape:** timestamp + vessel position +
source + payload + (later) analysis + embeddings + tags. That unified
schema is what makes the platform grow without rewrites.

---

## The flagship experience

The captain sits at the wheelhouse. The laptop beside the sounder monitor
runs **TZ Pro Agent**. There's a small icon in the system tray. Right-clicking
it opens a menu that does everything the captain needs:

- **Start/Stop Capture** — automatically follows TZ Pro lifecycle
- **Open Dashboard** — opens the 3-panel web UI in the default browser
- **Status** — NMEA fix quality, last capture time, satellite count
- **Open Captures Folder** — jump straight to today's folder
- **Quit**

From any phone or tablet on the boat's Wi-Fi, the crew opens the dashboard
at `http://proart.local:8090`. The same 3 panels:

- **LEFT — file tree** of captures organized by date, with distance filter
  (1mi / 5mi / 10mi / all) and date-range search at the bottom
- **CENTER — capture + analysis** with a markdown/code/raw-JSON view switcher
- **RIGHT — chatbot** that knows the whole collection, can be asked things
  like "compare last week's echograms to barometric pressure and look for
  correlations" or "what was the catch rate when we saw marks at 30-45 fm?"

The crew member can talk to their chatbot from the slush-ice station with
voice. The captain can show the same UI to a fisheries observer from the
couch in the cabin. The data lives on the ProArt; everyone else is just
looking at it.

That's the experience. Everything else is plumbing.

---

## Architectural spine (unchanging across all phases)

| Concern | Decision | Why |
|---|---|---|
| **Canonical data** | A unified `Moment` schema (timestamp + position + source + payload) | Lets DAW, marks, vector-DB, and spatial views all work off one truth |
| **Local storage** | SQLite as primary store, JSONL append-only log, raw files alongside | Queryable + human-readable + browsable + cheap backup |
| **Vector index** | Local first (nomic-embed via Ollama), cloud later (Workers AI / DeepInfra) | Privacy, offline, swap-in upgrade |
| **Key storage** | Windows DPAPI-encrypted vault, **excluded from backups** | Portability without leaking secrets |
| **Transport** | Local FastAPI on `0.0.0.0:8090`, swap to Cloudflare Worker in Phase 8 | Same frontend, different backend |
| **Provider abstraction** | One interface, many implementations (Ollama, Workers AI, DeepInfra, OpenRouter, OpenAI, Anthropic, custom) | Per-task model preference, fallback chains, no lock-in |
| **Multi-user** | Session IDs in URL, no auth at first, auth added when leaving the LAN | LAN-safe by default, Cloudflare-auth when going global |

---

## Phase index

| # | Name | Goal | Status |
|---|---|---|---|
| 1 | **Capture + Tray + LAN Dashboard + Provider Onboarding** | Local app working perfectly on the ProArt, reachable from any LAN device, with secure provider keys | **CURRENT** |
| 2 | Analyzer wiring | 10-min cadence runs small model first, escalates to heavy for selected moments | Planned |
| 3 | Multi-session chat | Crew invites, each session independent, shared underlying moments | Planned |
| 4 | Additional source feeds | Radar, AIS, autopilot, engine gauges via NMEA 2000 PGNs | Planned |
| 5 | Voice STT/TTS | Local Whisper for STT, DeepInfra/ElevenLabs for TTS | Planned |
| 6 | DAW timeline view | Horizontal time axis, lanes per source, click to jump into spatial view | Planned |
| 7 | Marks-as-output | Model produces TZ Pro marks (color/name/symbol/description) that write back | Planned |
| 8 | Cloudflare sync | SQLite→D1, files→R2, vectors→Vectorize, multi-device from anywhere | Planned |
| 9 | Vector-DB spatial/temporal | Embeddings encode position+time+source, "moments near this one" is a query | Planned |
| 10 | **Projection Layer (Theia/Blueprint)** | IDE-shaped runtime with four panels (left/center/right/bottom), widget model, layout presets, multi-device | **CURRENT DOCTRINE** (awaiting Phase 1.5 build) |
| ∞ | Platform-of-platforms | Feed-agnostic. OpenCPN, ROS for industrial robotics, anything that emits events | Vision |

See `phases/` for the full detail of each.

---

## Why "local-first, viral later"

A working local app is a **killer app** — the captain adopts it because
it's useful *today*, not because of a roadmap promise. The dashboard
actually shows the last week's marks when the barometer was dropping. The
chatbot actually answers questions about specific hauls. The voice capture
actually remembers what the crew said.

Once that's working on one boat, the same code works on every boat. The
same dashboard works for every captain. The same fleet-sync layer (Phase 8)
turns one boat's worth of work into a fleet-worth of intelligence without
any code changes.

That's when it goes viral — because the local experience is so good that
captains tell other captains, and those captains need the cloud to compare
notes. The cloud is the **federation layer**, not the foundation.

---

## Phasing principle

> Only the current phase is in focus during a build. Later phases are
> documented for vision but may pivot as discoveries reshape them.

We hold the vision loose. We hold the schema tight. Everything else is
plumbing we can rewrite.
