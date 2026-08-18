# Phase 1.5 — Static SPA Projection Layer

> **Status:** Active (supersedes Phase 10's "current doctrine" flag for the host decision)
> **Decided in:** Round 48 (Session #48), based on `THEIA_RESEARCH.md` R&D.
> **Depends on:** Phase 1 (dashboard.py on 0.0.0.0:8090).

## TL;DR

Replace the existing `dashboard.py` Jinja templates with a **static
single-page app** (Svelte 5) served by the same FastAPI process. Keep
all backend logic in Python; only the rendered HTML/CSS/JS becomes a
bundle. The four-panel IDE-shaped surface from `PROJECTION_LAYER.md`
lands here, but as a 50-100 KB Svelte bundle rather than a 500-700 MB
Theia install.

## Why we pivoted from Theia/Blueprint

| Constraint | Theia/Blueprint | Svelte SPA |
|---|---|---|
| Windows ProArt RAM (8 GB) | 6-8 GB baseline | <100 MB |
| Install footprint | 500-700 MB | 50-100 KB |
| Build toolchain | Node.js + npm + webpack | Node.js for build, none at runtime |
| Bridge to Python | Browser → Theia Node → FastAPI WS | Browser → FastAPI HTTP/WS directly |
| Layout mechanism | Custom drag-drop | Native CSS Grid (`grid-template: 2fr 1fr 1fr / 1fr 1fr`) |
| License | EPL-2.0 | MIT |

Theia was built for cloud dev environments; we have a boat laptop.
The SPA approach leaves 7+ GB of RAM free for capture, analyzer, and
the OS. See `THEIA_RESEARCH.md` for the full R&D trail.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Browser (any device on LAN)                             │
│                                                         │
│  Loads /ui/ from FastAPI → single Svelte bundle         │
│  Opens WebSocket to ws://host:8090/ws/agent             │
│  REST calls to /api/* for captures/chat/health          │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │ HTTP + WS
                          ▼
┌─────────────────────────────────────────────────────────┐
│ FastAPI (replaces dashboard.py) on 0.0.0.0:8090         │
│                                                         │
│  /            → serves /ui/index.html (Svelte mount)    │
│  /ui/*        → static files (bundle.js, bundle.css)     │
│  /api/health  → vessel + LAN + providers                 │
│  /api/chat    → POST, returns ChatResponse               │
│  /api/captures → list of recent captures                 │
│  /ws/agent    → bidirectional channel for live moments   │
│                                                         │
│  Mounts existing routes unchanged; only the renderer     │
│  changes from Jinja to SPA.                              │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │ same Python APIs
                          ▼
┌─────────────────────────────────────────────────────────┐
│ Existing Phase 1 layer (unchanged)                      │
│  bridge, providers, capture_daemon, tray_app, doctor    │
└─────────────────────────────────────────────────────────┘
```

## The four-panel layout (from PROJECTION_LAYER.md)

```
┌────────────────┬────────────────────────┬────────────────┐
│                │                        │                │
│  NAVIGATOR     │   FOCUS                │   AGENT TUI    │
│  (left)        │   (center)             │   (right)      │
│                │                        │                │
│  - Captures    │   - Sounder image      │   - Chat       │
│  - Moments     │   - Markdown render    │   - Hermes log │
│  - Anomalies   │   - Raw JSON toggle    │   - Status     │
│  - Filters     │   - Click-to-highlight │   - Quitting   │
│                │                        │                │
├────────────────┴────────────────────────┴────────────────┤
│                                                         │
│  PROJECTION (bottom)                                    │
│  - Waveforms, gauges, sparklines                        │
│  - This is NOT a terminal - it's the captain's          │
│    instrument cluster                                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

Layout is plain CSS Grid:

```css
.cockpit {
  display: grid;
  grid-template: 1fr auto / 280px 1fr 360px;
  grid-template-areas:
    "nav focus agent"
    "proj proj proj";
  height: 100vh;
}
```

## Components to build

In rough order of dependency:

1. **`web/` directory** — the Svelte project root
   - `web/package.json` — svelte 5 + vite 5 + nothing else
   - `web/src/App.svelte` — top-level `<div class="cockpit">`
   - `web/src/lib/Cockpit.svelte` — grid container
   - `web/src/lib/Navigator.svelte` — left panel
   - `web/src/lib/Focus.svelte` — center panel with tab switcher
   - `web/src/lib/AgentTUI.svelte` — right panel chat
   - `web/src/lib/Projection.svelte` — bottom panel
   - `web/src/lib/api.ts` — typed fetch wrapper for /api/* and /ws/agent

2. **`build_spa.py`** — small wrapper that:
   - Runs `npm run build` if `web/node_modules/` exists
   - Else outputs a single self-contained `index.html` with vanilla JS (zero-build fallback)
   - Writes the output to `static/ui/` so FastAPI can serve it

3. **`dashboard_spa.py`** — extends existing `dashboard.py`:
   - Mounts `StaticFiles(directory="static/ui", html=True)` at `/ui`
   - Adds `/ws/agent` WebSocket route (broadcasts new Moments to the UI)
   - Keeps all existing `/api/*` routes untouched

4. **`web/src/lib/WSBridge.ts`** — connects to `/ws/agent`, exposes a Svelte store

5. **Tests** — `scripts/_test_spa_layout.py`:
   - Smoke test the build produces valid HTML/JS bundle
   - Smoke test FastAPI serves `/ui/` and `/api/health` together
   - One Playwright test that loads the page and asserts four panels exist

## Exit criteria

- [ ] `python build_spa.py` produces a self-contained bundle ≤ 200 KB gzipped
- [ ] `python dashboard_spa.py` serves the bundle and all existing APIs
- [ ] Four panels render on a fresh load (no console errors)
- [ ] WebSocket `/ws/agent` pushes new Moments to the UI within 1s
- [ ] Existing `/api/chat` works unchanged from the new UI
- [ ] Phone-on-LAN smoke test: open `http://proart.local:8090/ui/` from a phone, see the four panels

## What this unlocks

- **Phase 2 analyzer** can write Moments directly to the WebSocket —
  the captain sees analysis appear live in the bottom projection panel.
- **Hermes the analyzer-hermit-crab** (see `docs/HERMES_TRAINING.md`)
  gets a UI surface to show her reasoning.
- **Multi-device** — the same bundle renders on helm laptop, cabin
  tablet, and phone; layout presets per device class.

## What we're explicitly NOT building

- Native Theia plugins (out of scope)
- Drag-and-drop panel rearranging (the four-panel shape is fixed; if
  we want flexibility, that's Phase 11)
- Server-side rendering of the SPA (static HTML is enough; the data
  comes through APIs)
- Authentication on the LAN (Phase 8 adds auth at the Cloudflare edge)

## See also

- `THEIA_RESEARCH.md` — the R&D that drove this pivot
- `architecture/PROJECTION_LAYER.md` — the IDE-shaped surface doctrine
- `architecture/DUAL_REPRESENTATION.md` — JSON truth, markdown render
- `docs/HERMES_TRAINING.md` — Hermes learning to analyze in this surface
- `docs/phases/phase-1.md` — the Phase 1 dashboard this replaces the renderer of