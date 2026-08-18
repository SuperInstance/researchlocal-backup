# Projection Layer — The Human Surface

> **Doctrinal status:** architectural spec, Phase 10 (current roadmap).
> **Read with:** `AGENT_OPERATING_MODEL.md` (for the IO contract),
> `architecture/CAPTURE_PIPELINE.md` (for what is being projected),
> `architecture/DUAL_REPRESENTATION.md` (for the JSON ⇄ markdown principle).

This document defines the **projection layer** — the surface where humans
and the agent see, click, point, and vibe-code together. It is the
visualization of the dual-representation principle, the runtime shape of
the cockpit hierarchy, and the answer to the question: *what does the
human actually look at?*

---

## TL;DR (one paragraph)

The captain and specialists communicate through **JSON files** (machine
truth) and **markdown** (human skim). The projection layer is the
**web-native, app-agnostic runtime** that renders those files as
panes a human can flip between, group by, annotate, and hand-edit
in place. It is structured like a developer IDE — **left panel /
center panel / bottom panel / right panel** — where every panel is a
**projection widget** registered against a data source. The right
panel is the agent. The bottom panel is whatever projection is most
useful at the moment (an echogram, a transcript, a JSON tree, a
control surface). Layouts are saveable presets. Presets can live in
separate browser tabs, windows, or monitors — so one laptop can be
running "analysis + echogram" on the helm monitor while a phone on
the bunk is running "controls + dashboard."

---

## The principle: the suit wears a face

The git-agent paradigm says the repo is the agent's body and the file
system is its memory. That body has no face. The projection layer is
the face.

- The agent's cockpit is the repo (text, JSON, markdown).
- The projection is a thin web app that renders cockpit files for
  humans and writes edits back as cockpit files.
- A projection that crashes does not lose work — the files are still
  on disk; the next projection that opens picks them up.
- A projection that lies (renders stale data) is broken by design;
  every widget reads from disk on load and reflects what the cockpit
  actually says.

This is the **same shape as a code editor**: VS Code, Theia, JetBrains
Fleet — they all render files, accept edits, and save back to disk.
We are building the fishing-equivalent of an IDE, where the "files"
are captures and analyses instead of source code.

---

## The IDE-shape: four panels, one model

The projection is structured exactly like a modern IDE. The panels
are not hardcoded to specific content; they are **slots** that
**projection widgets** register into.

```
+--------------------------------------------------------------+
| LEFT PANEL        |  CENTER PANEL                          |
| (navigation,      |  (active projection — togglable views) |
|  filters,         |                                         |
|  grouping)        |  [View: Image|JSON|Markdown|Transcript] |
|                   |                                         |
|  - Captures       |  +-------------------------------------+ |
|  - Anomalies      |  | (current view of selected moment)   | |
|  - Sessions       |  |                                     | |
|  - Engines        |  |                                     | |
|  - Trip logs      |  |                                     | |
|                   |  |                                     | |
|                   |  |                                     | |
|                   |  +-------------------------------------+ |
|                   |                                         |
+-------------------+-----------------------------------------+
| RIGHT PANEL       |  BOTTOM PANEL                           |
| (agent — TUI or   |  (projection — IDE "terminal" slot,    |
|  rich chat)       |   but not a shell by default)           |
|                   |                                         |
|  > _              |  [echogram | transcript | controls |    |
|                   |   dashboard | shell | …]                |
|  agent:           |                                         |
|  "highlight the   |  +-------------------------------------+ |
|   schools on the  |  | (current projection)                | |
|   last hour"      |  |                                     | |
|                   |  |                                     | |
|                   |  +-------------------------------------+ |
+-------------------+-----------------------------------------+
```

The shape is fixed; the **content of each panel is a runtime choice.**

### Left panel — the navigator

The left panel is a **tree/filter/group-by surface** over the cockpit's
captures and analyses. It is the question *what do you want to see?*

| Grouping | Use when |
|---|---|
| **Time** (last hour, last trip, today, yesterday) | the captain is paging through recent work |
| **Date** | comparing across days of the season |
| **Location** (lat/lon cell, depth zone, structure) | the captain is fishing a known spot |
| **Tide** (rising, falling, slack, by hour) | the captain is fishing a tide change |
| **Engine state** (RPM band, load, temp regime) | the engineer is reviewing propulsion |
| **Anomaly class** (thermal hotspot, audio knock, NMEA dropout) | the captain is investigating something specific |

If the human wants a grouping the panel doesn't offer (e.g. "all
captures within 0.5 nm of this waypoint during flood tide"), they
ask the right-panel agent — the agent either adds the grouping as
a left-panel widget (vibe-coded live, see below) or produces the
filtered set as JSON and drops it into the center panel.

### Center panel — the focus

The center panel renders **one selected moment or artifact** at a
time. It is the answer to *what is this thing?*

Views are **toggleable, not exclusive** — you can have all three open
side-by-side if you want.

| View | Source | Renders |
|---|---|---|
| **Image** | `captures/{date}/{HHMMSS}.{ext}` | the raw echogram, thermal frame, audio spectrogram, photo |
| **JSON** | `captures/{date}/analysis/{HHMMSS}.json` | the machine-truth analysis with field-level highlighting |
| **Markdown** | `captures/{date}/analysis/{HHMMSS}.md` | the pre-rendered human-skim version |
| **Transcript** | `captures/{date}/conversation/{HHMMSS}.jsonl` | the right-panel conversation that referenced this moment |

**Click-to-highlight** is the synoptic primitive. Click a school in
the echogram view; the JSON view highlights the `features` field that
describes it; the markdown view scrolls to the anchor that mentions it.
Click a `confidence` field in the JSON; the markdown paragraph that
discussed it highlights; the echogram overlays the bounding box. This
is how a captain who is paging fast can find what matters.

The center pane is also where **edits happen**. When the human edits a
JSON field or annotates a markdown paragraph, the edit:

1. Is validated against the schema in
   `architecture/CAPTURE_PIPELINE.md`.
2. Is appended to `analysis/{HHMMSS}.edits.jsonl` (append-only audit).
3. Bumps the JSON `version` and regenerates the markdown.
4. Queues the moment into the retraining loop.

### Right panel — the agent

The right panel is the **copilot** (human-facing LLM agent), which is
in turn a projection of the **captain** (orchestrator) over the
**specialists**. It is the question *what should we do about it?*

- The agent can **filter the left panel** ("show me the last hour of
  troll-speed moments with thermal hotspots").
- The agent can **re-analyze the center selection** ("re-score this
  school against the new bait model").
- The agent can **vibe-code new widgets** ("add a grouping by depth
  band to the left panel") and the new widget registers itself.
- The agent can **speak** ("tell me when the port exhaust runs 5°F
  above normal for 10 minutes") and the captain schedules the
  watch.

This is not a chatbot sidebar. The right panel is a **TUI-grade
surface** — it accepts commands, produces structured output, and
mutates cockpit state. The chat appearance is a convenience; the
substance is file ops.

### Bottom panel — the projection (replacing the IDE terminal)

This is the insight that motivates the whole pivot. In a normal IDE,
the bottom panel is a shell terminal. In the boat-agent projection,
the bottom panel is **whatever projection is most useful at the
moment** — and it is normally **not a shell.**

| Bottom-pane projection | When it's the right answer |
|---|---|
| **Echogram timeline** | fishing; the captain wants to scroll the last hour |
| **Conversation transcript** | reviewing what the agent said earlier |
| **Controls** (engine, navigation, lights, pumps) | at the helm; manual control + agent's last advice |
| **Dashboard** (KPIs, gauges, alerts) | in the cabin or off-watch |
| **Shell** | only when debugging the projection itself |

**The terminal is demoted.** The shell is still available (the
projection runs in a node, and the captain can spawn a shell
session for plumbing) but it is one widget among many, not the
default. This matches the git-agent paradigm: the agent reads
files, the human reads files, the shell is plumbing.

---

## Layout presets — saveable, portable, multi-device

Layouts are first-class saved state. A layout is:

```json
{
  "name": "Fishing — analysis + echogram",
  "left": {"widgets": ["captures.tree"], "width": 240},
  "center": {"widgets": ["capture.image", "analysis.markdown"], "split": "horizontal"},
  "right": {"widgets": ["agent.tui"], "width": 360},
  "bottom": {"widgets": ["echogram.timeline"], "height": 320},
  "context": "trip:2026-07-24.salmon"
}
```

Layouts live in `~/tzpro-personal/layouts/` (private; not in the
suit). They can be:

- **Saved per task** ("this is the layout I use at the helm").
- **Saved per device** ("this is the layout on the cabin tablet").
- **Shared with specialists** ("the cascade agent gets the
  anomaly-review layout").
- **Synced across devices** (when Starlink is up, layouts ride the
  cloud sync from `architecture/PRIVACY_BOUNDARY.md`).

Different layouts run in different browser windows. The captain can
have:

- **Helm monitor** — controls + echogram timeline (touch-friendly).
- **Cabin laptop** — analysis + JSON + markdown (full review).
- **Phone on the bunk** — alerts + agent TUI (off-watch).

All three are projections of the same cockpit. Same files, different
renders, one source of truth.

---

## Widget model: every panel is a registered projection

A **widget** is a small piece of UI that knows how to render one
kind of cockpit data and how to write edits back. Widgets are
registered, not hardcoded.

```ts
interface Widget<I, O> {
  id: string;            // "echogram.timeline"
  inputs: I;             // the file paths and config it renders
  render(host: HostNode): Disposable;
  onEdit?(change: Edit): Promise<void>;
  cost: "low" | "medium" | "high";  // for the cost-throttle
}
```

| Widget | Inputs | Renders | Writes |
|---|---|---|---|
| `captures.tree` | `captures/**/*.json` | left-panel nav | selection events |
| `analysis.json` | `analysis/{m}.json` | center JSON view | field-level edits |
| `analysis.markdown` | `analysis/{m}.md` | center markdown view | paragraph edits |
| `echogram.timeline` | `captures/{date}/*.png` | bottom echogram scroll | bookmark writes |
| `engine.controls` | NMEA + relays | bottom controls panel | relay commands |
| `agent.tui` | cockpit + delegation | right panel | task dispatches |
| `dashboard.gauges` | `vessel_state.jsonl` | bottom KPI tiles | alert config |

**Widget manifests are JSON files in `projection/widgets/`.** A
widget can be added by dropping a manifest + a build artifact into
that folder. The projection host picks them up on reload. This is
how the agent **vibe-codes new widgets live** — it writes a
manifest, the projection registers it, the human sees it appear.

---

## Runtime: Theia (or Blueprint) as the host

The projection runs in a **web-native** shell because:

- **Compatibility** is not a problem (any device with a browser).
- **Multi-device** is trivial (the same URL on every device).
- **Vibe-coding new widgets** is the agent's wheelhouse
  (TypeScript + a JSON manifest).
- **No native install** on the helm, the phone, the cabin laptop.

The candidate host is **Theia** (Eclipse) — an open-source IDE
framework already shaped like this. Alternative: **Blueprint** (a
Theia sub-project that strips out the IDE assumptions and gives a
clean application shell). The choice is deferred to
`architecture/THEIA_RESEARCH.md`; the projection contract above
holds either way.

The host:

1. Serves the four-panel layout shell.
2. Loads widget manifests from `projection/widgets/`.
3. Reads cockpit files via a thin JSON-RPC bridge to the agent
   service.
4. Streams edits back through the same bridge.

The agent service (Python) is the source of truth for the cockpit;
the projection (TS/JS) is a pure renderer. The split mirrors the
git-agent paradigm: agent in its own cockpit, projection as an
adapter, never the other way around.

---

## The projection's contract (and what it must NOT do)

The projection is an **adapter**, not the cockpit. It must:

- Read cockpit files for state.
- Write cockpit files for edits (through schema-validated APIs).
- Reflect disk state on every load (no projection-side caches that
  lie about what the cockpit says).

It must **not**:

- Maintain its own database (the cockpit's SQLite is canonical).
- Run long-lived analysis (the agent service owns this).
- Make autonomous decisions (it asks the right-panel agent, which
  asks the captain, which dispatches a specialist).
- Hold secrets (the vault is in `~/tzpro-personal/vault/`; the
  projection reads them via the agent service with the captain's
  permission).

A projection that wants to do any of these things is a sign that
the cockpit should grow a new specialist — not that the projection
should grow a side brain.

---

## Phasing

| Phase | What lands |
|---|---|
| **Phase 1 (done)** | `dashboard.py` — minimal FastAPI web UI, JSON over HTTP, captures list |
| **Phase 1.5** | `projection_host/` — Blueprint + four-panel shell, wraps `dashboard.py` as a widget, no new widgets yet |
| **Phase 10.0** | Widget model formalized; `captures.tree`, `analysis.json`, `analysis.markdown`, `echogram.timeline`, `engine.controls` ship |
| **Phase 10.1** | Click-to-highlight across views; layout presets |
| **Phase 10.2** | Multi-device layouts (cabin laptop, phone, helm monitor) |
| **Phase 10.3** | Cloud-hosted projection (Oracle / Cloudflare + Starlink) for shore access |
| **Phase 10.4** | Vibe-coding live widget generation via right-panel agent |

The current `dashboard.py` is preserved through all phases. It
becomes one widget (`dashboard.gauges`) inside the projection
host, not the surface itself.

---

## Migration: how an existing `dashboard.py` user lands here

Nothing breaks for the human on day one of Phase 1.5:

1. The projection host serves `dashboard.gauges` as the **only
   widget** in the bottom panel.
2. The left, center, and right panels are present but empty.
3. Over Phase 10, widgets appear as they ship. The captain never
   has to "switch to the new UI" — the new UI shows up *around* the
   old one, panel by panel.

This is the migration philosophy of the whole system: **the
projection accretes around the captain**, never replacing.

---

## See also

- `docs/AGENT_OPERATING_MODEL.md` — the IO contract and cockpit
  hierarchy that this projection serves.
- `docs/NAVIGATION.md` — `phase-2` and `doctrine` scopes include
  this doc from `Round 14` onward.
- `docs/architecture/CAPTURE_PIPELINE.md` — what is being
  projected (the senses).
- `docs/architecture/DUAL_REPRESENTATION.md` — JSON ⇄ markdown
  principle that the center panel toggles between.
- `docs/architecture/PRIVACY_BOUNDARY.md` — layouts live in
  `~/tzpro-personal/layouts/`, not in the suit.
- `docs/architecture/THEIA_RESEARCH.md` — host research note
  (forthcoming, Phase 1.5).
- `docs/ROADMAP.md` — Phase 10 in the phase index.
