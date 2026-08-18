# Scrubber UX Compliance Checklist + Smoke Test Plan

> **Target Audience:** Auditor running acceptance against the Week-2 scrubber
> build (docs/22) and the captain on the boat.
> **Status:** Governing for the Week-2 audit · Date: 2026-07-19
> **Sources:** docs/22 (architecture, endpoints, build order), docs/23
> (research-backed principles U1–U7, T1–T5, A1–A7; risks R1–R5).

This document has three parts:

1. **Part A** — A 25-item PASS/FAIL checklist an auditor runs against the
   built scrubber. Organized by: (A.1) docs/23 evidence rules, (A.2)
   docs/22 endpoint contract + performance budgets, (A.3) trust rules.
2. **Part B** — A 10-minute manual smoke-test script for the captain on
   the boat. Open → scrub → find a school → keyboard-only → close+reopen
   state persistence.
3. **Part C** — The 5 most likely places the implementation will cheat or
   cut corners, and how to detect each.

Pass criterion for the Week-2 audit (docs/21): **captain scrubs a 12-hour
day in 2 minutes, unassisted.** Every Part A item must PASS or be
documented as an explicit defer with a docs/13 open-question reference.

---

## Part A — Compliance Checklist (25 items)

Mark each item **PASS / FAIL / DEFER (with Q-id)**. A FAIL on any item
in sections A.1 or A.3 blocks the Week-2 acceptance; a FAIL on A.2
performance items blocks the performance budget.

### A.1 — docs/23 evidence rules (items 1–13)

| #  | Check | Rule of origin | How to verify |
|----|-------|----------------|---------------|
| 1  | **Primary targets ≥ 64 px.** Transport buttons (⏯ ⏪⏩), next/prev-event, preset chips, A/B loop handles all measure ≥ 64 px on a rendered screenshot. | docs/23 U4 | Screenshot at native DPI; measure bounding box of each labeled control with a pixel ruler; reject anything < 64 px. |
| 2  | **Nothing interactive < 48 px.** Every clickable/draggable element has a minimum bounding box of 48 px and dead space between adjacent targets. | docs/23 U4 | Grep the DOM/canvas for `width < 48` or `height < 48` on interactive nodes; visual audit of any canvas-drawn hit regions. |
| 3  | **Variable-rate drag scrubbing.** Drag along the timeline scrubs by time; dragging the finger *downward* while held reduces rate (finer scrubbing). Releasing returns to base rate. | docs/23 T1, docs/22 amendment | Instrumented test: drag 200 px right, record cursor ts; drag 200 px right while offset 100 px down, record ts. The downward-offset case must cover *less* time per pixel. No constant-rate slider ships. |
| 4  | **No jog-wheel UI anywhere.** There is no rotary knob, shuttle ring, jog dial, or skeuomorphic transport widget. The only scrubbing surface is the timeline. | docs/23 anti-pattern, docs/22 amendment | Visual + DOM grep for `<input type="range">`-styled-as-knob, SVG circles with rotation handlers, or canvas-drawn arcs. |
| 5  | **No fisheye distortion.** The whole-day bar and the zoomed-window strip are both linear in time. There is no compression of time near the cursor. | docs/23 T4 | Pixel-time calibration: pick three tick pairs whose true time delta is known; measure pixel distance on both bars; ratio must be constant (no compression). |
| 6  | **Keyboard fallback for every function.** ⏯/⏪/⏩, next/prev-event, A/B loop set/clear, preset chips, layer sliders, day pick — all reachable by keyboard. Visible focus rings. | docs/23 U6 | Run the keyboard-only path in Part B below. Confirm every action listed in HUD bar has a documented keybind; confirm Tab order is logical; confirm focus ring is ≥ 2 px and high-contrast. |
| 7  | **Provenance-separated event tracks.** Machine marks (schools, anomalies) and human marks (deck presets, labels) render on *separate* horizontal tracks with visually distinct weights/icons/colors. | docs/23 T3, docs/22 amendment | Render a synthetic day with at least one of each track; verify two distinct y-bands and that the legend distinguishes "AI / machine" vs "captain / human." |
| 8  | **"+N" clustering on overflow.** When two or more ticks fall within the current pixel resolution, they collapse to a single "+N" badge that expands on zoom-in. | docs/23 T3 | Synthetic timeline with 6 ticks within 8 px → must render 1 tick + "×6" label that expands when zoom ≥ 4×. |
| 9  | **Highlight cursor with configurable lead/lag.** The "holy shit" highlight (docs/22) renders as an A/B loop with a *visible* lead-in (before) and lag-out (after) the peak novelty. Both values are user-configurable. | docs/23 T5, docs/22 amendment | Open the highlight; verify the A/B handles bracket more than just the peak frame. Open settings; verify lead and lag have numeric inputs (or sliders) and the default values are not 0/0. |
| 10 | **Dark high-contrast styling.** Default palette is dark and high-contrast (OpenBridge Night or equivalent). Body text contrast ≥ 7:1 against background. Echogram stays dark; chrome is high-contrast. | docs/23 U1, U2 | Run WebAIM contrast checker on text/background pairs from a screenshot. Eyeball: any gray-on-gray fails immediately. |
| 11 | **No red/green-only semantics.** No state is encoded by red vs green *alone*; every such signal also carries shape, label, or position. | docs/23 (alarm fatigue), docs/22 build-order AC | Grep the design for red/green pairs; verify each carries a secondary channel (icon, text, position). Simulate red-green colorblindness (Chrome DevTools rendering) and confirm signals remain distinguishable. |
| 12 | **Two-level overview+detail, linear time.** A whole-day bar at the top and a zoomed-window strip below (or equivalent). Both linear. No third distortion level. | docs/23 T4 | Visual + item 5 calibration on both bars. |
| 13 | **Frame-at-cursor ≤ 100 ms warm.** After the first `/api/day` call, dragging the cursor to a new frame returns the PNG in under 100 ms. | docs/22 build-order AC #2 | See A.2 item 16 for the measurement protocol. |

### A.2 — docs/22 endpoint contract + performance budgets (items 14–19)

| #  | Check | Rule of origin | How to verify |
|----|-------|----------------|---------------|
| 14 | **Endpoint `GET /api/day/<YYYY-MM-DD>` exists** and returns `{frames: [...], records: [...]}` where each frame carries `frame_id, ts_utc, lat, lon, sog, cog, sha256, tier, novelty, keep_reason`. | docs/22 "Endpoints (the whole contract)" | `curl -s localhost:<port>/api/day/2026-07-19 \| jq '.frames[0] \| keys'` — must include all listed fields. |
| 15 | **`GET /api/day` < 300 ms for a full 24 h day.** Cold-cache measurement on a representative 24 h day (144 frames at 10-min cadence, full record join). | docs/22 performance budgets | `curl -w '%{time_total}\n' -o /dev/null -s localhost:<port>/api/day/<date>` — repeat 5× after a cold restart; report median. |
| 16 | **`frame_at(ts)` ≤ 100 ms warm.** With the day's frames already in memory/client cache, fetching the blob for a new cursor position returns in < 100 ms. | docs/22 build-order AC #2 | Warm the day, then `curl -w '%{time_total}\n' -o /dev/null -s localhost:<port>/api/blob/<sha>` for 10 different cursor positions; report median. |
| 17 | **`GET /api/blob/<sha256>` returns the PNG** and the blob is byte-identical to `blobs/<sha>.png` on disk (or a cold-tier pointer that resolves). | docs/22 endpoint contract | `curl -s localhost:<port>/api/blob/<sha> -o /tmp/got.png`; `sha256sum /tmp/got.png blobs/<sha>.png` — must match. |
| 18 | **`GET /api/day/<date>/highlight` returns the highlight cursor.** Returns `{cursor_ts, lead_s, lag_s, caption, frame_id, conf_hint}` (or equivalent). The cursor lands on the day's highest-novelty moment with a confident record joined. | docs/22 "holy shit" moment | `curl -s localhost:<port>/api/day/<date>/highlight \| jq .` — verify the fields and that `cursor_ts` corresponds to the frame with `max(novelty)` joined to a record. |
| 19 | **Trace sync error < 1 frame.** When the cursor moves, the displayed frame's `ts_utc` and the timeline tick align within one frame's duration. | docs/22 performance budgets | Instrument: drive the cursor to 10 known timestamps; compare displayed `ts_utc` to the target. Max delta in seconds must be < one frame's duration. |

### A.3 — Trust rules (items 20–25)

| #  | Check | Rule of origin | How to verify |
|----|-------|----------------|---------------|
| 20 | **Highlight caption uses calibrated language, not raw model confidence.** The one-line caption above the highlight (docs/22: "14:28 — school at 26 fm, hard bottom at 50 fm, 91% conf.") MUST be replaced or augmented with a calibrated hit-rate form (e.g., "17 of my last 20 calls like this held"). The raw model score "91% conf" must not appear in the captain-facing caption. | docs/23 A1, R1 | Screenshot the highlight caption; grep the response payload for `conf` / `confidence` / `%` in any captain-facing field. The presence of a bare model confidence in the captain view = FAIL. |
| 21 | **No raw model score anywhere in the captain surface.** Every visible confidence-like number is a calibrated track-record statistic with a denominator ("17 of 20"), not a raw probability. | docs/23 A1 | Sweep the rendered DOM and visible canvas text for `%` patterns; confirm each is a hit-rate (numerator + denominator). Auditor-only screens (post-login, separate route) may show raw scores. |
| 22 | **Effective autonomy is the primary readout (not dial position).** If the scrubber surfaces any autonomy/mode indicator, it shows what the system is *doing*, with the gap and its cause visible. | docs/23 A4, R4 | Visual + copy audit: the label is "Doing X" or equivalent, not "Mode 2." If a gap exists, a cause field is shown. |
| 23 | **Recommendation is behind a tap (evidence first).** No escalation-style UI surfaces a recommendation above the evidence. If recommendations exist in the scrubber, the evidence precedes them by at least one interaction. | docs/23 A5, R3 | Walk any escalation card from top to bottom: first content is evidence (frames, ticks, calibrated track record); recommendation is below the fold or behind a "Show recommendation" tap. |
| 24 | **Alarm/annunciation hygiene.** Every audible or visual alert in the scrubber (a) tells the captain what is happening, (b) tells him why, (c) tells him what to do. There are no bare notifications. | docs/23 (EEMUA 191 / ISA-18.2) | Catalog every alert: each must have all three fields. Count the alerts in any 10-minute window; if more than 3, the implementation is alarm-flooding (FAIL pending redesign). |
| 25 | **Vocabulary is domain-verified.** Labels like "school," "hard bottom," "fm" are used consistently and match the captain's vocabulary (docs/23 A7). No generic ML jargon ("anomaly score," "detection") appears in captain-facing copy. | docs/23 A7 | Read every captain-facing string; grep for `score`, `confidence`, `detection`, `prediction`, `model`. Each hit = FAIL or DEFER with a glossary mapping. |

---

## Part B — 10-minute Captain Smoke Test (on the boat)

The captain performs these steps unaided. The test passes if every step
completes within the cumulative 10-minute budget and the captain
correctly identifies the day's "holy shit" moment without prompting.

**Setup.** The scrubber is opened at `localhost:<port>` in Edge kiosk
mode on the wheelhouse panel. A representative 24 h day is pre-loaded
(`/api/day/2026-07-19` returns 144 frames + records; at least one
machine-marked school and one human-marked event exist on that day).

### Step 1 — Open (≤ 60 s)

1. Captain opens the scrubber.
2. **PASS criterion:** within 3 s, the scrubber renders with the
   cursor on the day's highest-novelty moment, the A/B loop visible,
   the highlight caption displayed at the top, and the day's first
   frame already painted in the frame pane.
3. Captain reads the highlight caption aloud. It must be calibrated
   language (no "X% conf"), and it must name a real, verifiable moment.

### Step 2 — Scrub (≤ 3 min)

1. Captain drags the timeline cursor left to right across the full day.
2. **PASS criterion:** drag is smooth; vertical offset while dragging
   visibly *slows* the rate (variable-rate drag, item 3).
3. Captain drags to ~06:00 and to ~18:00 and confirms the frame
   updates at each position.
4. Captain verifies: HUD bar (clock, lat/lon, sog, cog) updates to
   match the cursor's frame within 100 ms.

### Step 3 — Find a school (≤ 3 min)

1. Captain uses the next-event button (64 px) to jump forward through
   machine marks (schools).
2. **PASS criterion:** each tap advances the cursor to the *next*
   machine-track tick and stops. Pressing the button again advances
   to the one after.
3. Captain switches to the human-marks track (separate band) and
   uses next-event again.
4. Captain confirms the two tracks are visually separate and that
   pressing next-event does *not* land on a tick from the other track.
5. Captain zooms the timeline 4×. Where multiple ticks previously
   overlapped, a "+N" badge appears; zooming further expands it
   (item 8).

### Step 4 — Keyboard-only (≤ 2 min)

1. Captain lifts his hands off the trackpad/touchscreen and operates
   only the keyboard.
2. **PASS criterion:** every action from steps 1–3 above is reachable
   via documented keys. Specifically:
   - `←` / `→` scrub by one frame (or by the current zoom step)
   - `Shift+←` / `Shift+→` jump to previous / next event
   - `Space` toggles play/pause
   - `0` / `1` / `2` / `3` set dial-equivalent modes (if surfaced)
   - `[` / `]` set the A/B loop in/out points
   - `Tab` cycles through interactive controls with a visible focus ring
3. Captain finds the day's school using keyboard only. If he cannot,
   the keyboard fallback is incomplete (item 6 FAIL).

### Step 5 — Close + reopen state (≤ 90 s)

1. Captain moves the cursor to a known non-default position (e.g.,
   11:42), changes the overlay opacity to ~40 %, selects the
   "Standard" preset chip, and closes the scrubber (browser close,
   not refresh).
2. Captain reopens the scrubber.
3. **PASS criterion:** within 3 s, the scrubber returns to:
   - cursor at 11:42
   - overlay opacity at ~40 %
   - Standard preset active
   - same day loaded
   - highlight NOT auto-restored (state = "where I left off," not
     "where the system wants me")
4. Captain confirms via localStorage inspection that the persisted
   keys are human-readable (audit trail).

**Total budget:** 10 min. **Overall PASS** = all five steps PASS and
total elapsed ≤ 10 min. **Overall FAIL** = any step fails the criterion,
or the captain needs prompting at any point.

---

## Part C — Five Most Likely Places the Implementation Will Cheat or Cut Corners

These are the corners where the build will most plausibly regress the
spec. Each cheat has a name, the temptation, the signature in the code
or UI, and a detection recipe.

### C.1 — The "confidence%" cheat (item 20 / R1)

**The temptation.** docs/22's example highlight caption explicitly says
"91% conf." A copy-paste of that example into the live UI is the
single fastest regression. It also satisfies a PM-looking stakeholder
who wants "a number" in the caption.

**Signature.** Any visible string of the form `\d{1,3}%` in a
captain-facing field whose semantic source is the model's raw score
(`frames.novelty` × record confidence, or similar). Especially common
in the highlight caption, the per-frame metadata popover, and the
next/prev-event tooltip.

**Detection.**
1. Open the highlight on three different days; `grep -E '\d+%'`
   every visible string. Each hit: classify numerator/denominator.
2. Inspect the network response for `/api/day/<date>/highlight`. If
   the payload carries a field named `confidence`, `conf`, `score`,
   `novelty`, or `prob` and the UI renders it untransformed: FAIL.
3. Run a regex sweep across the rendered DOM for the pattern
   `\\b\\d{1,3}%\\b` after stripping units like `100%` battery;
   classify each surviving hit as calibrated (has denominator) or
   raw (no denominator).

### C.2 — The "we shipped a jog dial" cheat (item 4 / T1 anti-pattern)

**The temptation.** The captain is used to jog wheels from video
editing and some marine MFDs. A junior implementer will add one as
"intuitive." The research is unambiguous: this is the strongest
negative result in docs/23.

**Signature.** An SVG `<circle>` with a rotation handler, a
stylized `<input type="range">` with `border-radius: 50%`, or any
canvas-drawn arc with mousedown/mousemove handlers that translate
angular delta to time delta.

**Detection.**
1. Visual audit: any circular control near the timeline = FAIL.
2. DOM/canvas grep: `transform: rotate(`, `getBoundingClientRect`
   returning equal width and height on a non-button, or canvas arc
   draw calls following mouse-move deltas computed as atan2 — all
   FAIL.
3. Functional test: try to scrub by *rotating*. If anything happens
   that is not "drag along the timeline" or "drag down for finer
   rate," FAIL.

### C.3 — The "targets are 44 px" cheat (items 1, 2 / U4)

**The temptation.** The original docs/20 floor was 44 px. docs/23
U4 raised it to 64 px primary / 48 px floor. A build that started
against docs/20 and was only partially amended will ship 44 px
targets, especially for the transport buttons and next/prev-event.

**Signature.** Transport buttons (⏯ ⏪⏩), the next/prev-event
buttons, and the A/B loop handles measured at < 64 px on screen.
Often the SVG icon is 64 px but the clickable bounding box is the
icon's transparent interior plus a small padding.

**Detection.**
1. DevTools: select each primary control, read its rendered
   `getBoundingClientRect()`.
2. Screenshot at native DPI; measure with a pixel ruler.
3. Specifically test the next-event and A/B-loop buttons: these are
   the controls the captain hammers with a braced thumb in a seaway
   and they must hit the 64 px floor with no excuses.

### C.4 — The "we'll just dump both event types on one track" cheat (items 7, 8 / T3)

**The temptation.** Two tracks mean two y-bands, two legends, two
hit-test regions, and a clustering policy that respects both. One
track means one CSS class and one `for` loop. A time-pressed
implementer will ship one track and call it "T3" because there are
*two* colors.

**Signature.** A single timeline row where machine marks and human
marks differ only by color; or two rows where the legend does not
distinguish provenance; or a "+N" cluster that merges ticks from
*both* tracks (violating T3's adaptive clustering, which is
provenance-aware).

**Detection.**
1. Synthetic day with one machine tick at t=10:00:00 and one human
   tick at t=10:00:01. At default zoom: they must render on *different*
   y-bands. If they overlap or stack on one row: FAIL.
2. Render the legend. If "machine" and "human" (or equivalent
   verified-vocabulary terms) are not both labeled: FAIL.
3. Force a cluster: 6 ticks within 8 px from *both* tracks. The
   "+N" badge must show *two* clusters (one per provenance), not
   one merged cluster of 12.

### C.5 — The "state doesn't actually persist" cheat (Part B step 5 / docs/22 AC #4)

**The temptation.** Persistence requires writing to localStorage on
every state change, restoring on load, deciding what to persist (day,
cursor ts, overlay opacity, preset, A/B loop, zoom), and handling
schema migration. It is invisible work that the captain only notices
when it's broken. A build that "remembers the day" but resets the
cursor to the highlight on reopen will pass a casual review and fail
the smoke test.

**Signature.** Reopen behavior that:
- always lands on the highlight instead of where the captain left
  the cursor;
- resets overlay opacity to 100 % and preset to "Minimal";
- clears the A/B loop;
- forgets the zoom level.
Also: localStorage keys that are JSON-blob and unreadable (no audit
trail); or localStorage writes that fire only on `beforeunload` and
therefore miss crashes/forced closes.

**Detection.**
1. Run Part B Step 5 verbatim.
2. In DevTools, watch localStorage writes during interaction. Writes
   must occur on state change, not only on unload.
3. Inspect the persisted keys; they should be human-readable and
   versioned (e.g., `scrubber.v1.day`, `scrubber.v1.cursorTs`).
4. Forcibly kill the browser tab mid-session (not graceful close);
   reopen. The most recent state must still be there (or, at worst,
   the last debounced write within ~500 ms).
5. Schema-migration test: write a v0 localStorage blob, load, confirm
   the scrubber either migrates or surfaces a clean reset — never
   throws.

---

## Audit signature

At the end of the audit, the auditor signs:

```
Auditor: ______________________  Date: __________
Part A:  ____ / 25 PASS    ____ DEFER (Q-ids: __________)    ____ FAIL
Part B:  PASS / FAIL   Elapsed: _____ min
Part C:  Cheats detected: C.1 [ ] C.2 [ ] C.3 [ ] C.4 [ ] C.5 [ ]
Week-2 acceptance (docs/21): PASS / FAIL
```

---

**Cross-references:** docs/20 (UX rules, trust budget), docs/21
(Week-2 acceptance criterion), docs/22 (architecture, endpoints, build
order, amendment), docs/23 (research-backed principles U1–U7, T1–T5,
A1–A7; risks R1–R5).