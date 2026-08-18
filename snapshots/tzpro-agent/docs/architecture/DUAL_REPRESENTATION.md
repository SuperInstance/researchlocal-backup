# Dual Representation — JSON as Truth, Markdown as Render

> **Doctrinal status:** architectural spec, cross-cutting (consumed by
> the capture pipeline, the analyzer, and the projection layer).
> **Read with:** `architecture/PROJECTION_LAYER.md` (the runtime that
> toggles between the two), `architecture/CAPTURE_PIPELINE.md` (the
> `Moment` and `MomentAnalysis` schemas this elaborates),
> `AGENT_OPERATING_MODEL.md` (the IO contract).

This document defines the **dual-representation principle**: every
analysis the agent produces is stored as a **JSON file** (machine
truth, with tensors, vectors, importance weights, and correlations as
first-class numbers) and **rendered to a markdown file** for human
skim. The markdown is a deterministic function of the JSON. The JSON
is canonical; the markdown is its projection for human eyes.

---

## TL;DR (one paragraph)

A2A-native **JSON** is the source of truth — parseable, diff-friendly,
schema-validated, with numbers as first-class (floats for
probabilities, lists for vectors, dicts for tensors, references to
other files for cross-source correlations). **Markdown** is a
deterministic render of the JSON, regenerated every time the JSON
changes, optimized for a human paging fast through the latest
analyses. The human usually skims the markdown; when something
catches the eye they stop and read it; when they need to inspect
the machine reasoning they switch the center pane to JSON. The
center pane can have all three views (image, JSON, markdown) open
side-by-side with **click-to-highlight synoptics**: click a school
in the image, the JSON field that describes it lights up, the
markdown anchor that mentions it scrolls into view. The human can
also **edit** either representation — the markdown paragraph or
the JSON field — and the other regenerates. Every edit is
appended to a per-moment audit log, bumping the version, and
queues the moment for the retraining loop. The principle scales
because both representations are **lossless round-trips** of the
same underlying truth.

---

## Why dual representation

The agent produces analyses that are:

- **Numbers** (probabilities, distances, vectors, deltas)
- **Cross-references** (this school correlates with that AIS contact
  200m away, with weight 0.78)
- **Reasoning chains** (because the bait model says X, and the
  thermal anomaly says Y, the conclusion is Z)
- **Alternatives considered** (rejected with scores, so the human
  can audit why the agent picked what it picked)

All of these **lose information when flattened into prose.**
NLP descriptions of vector similarities, importance weights, and
correlations are like translating a spectrogram into words —
something is preserved, but a lot is lost.

Meanwhile, the human paging through the last hour of fishing
**wants to skim fast** — they want the markdown headline, not
the JSON tree. They want a glance that says "8 schools, three
worth investigating, the second one is on a temperature break
matching last week's hot spot."

These two consumers want different shapes. The right answer is to
serve both, with the same canonical truth underneath.

---

## The shape: `MomentAnalysis` JSON

The analysis is an **A2A-native JSON object** with first-class
numerical fields. Sketch (see `architecture/CAPTURE_PIPELINE.md`
for the full `Moment` schema and how `analysis` hangs off it):

```json
{
  "moment_id": "2026-07-23T14:32:18Z",
  "version": 3,
  "edit_count": 2,
  "schema": "moment-analysis/v1",

  "source": "echogram.capture",
  "produced_by": "granite4.1:8b@ollama",
  "produced_at": "2026-07-23T14:35:02Z",
  "confidence": 0.82,
  "cost_tokens_in": 1840,
  "cost_tokens_out": 412,

  "features": {
    "school_count": 8,
    "school_sizes_fm": [12, 18, 6, 4, 22, 9, 14, 7],
    "depth_band_fm": {"min": 18, "max": 42, "median": 27},
    "bottom_structure": "shelf_edge",
    "thermal_break": true,
    "thermal_delta_c": 1.8
  },

  "embeddings": [0.013, -0.227, 0.481, ...],

  "correlations": [
    {
      "with_moment": "2026-07-23T14:30:02Z",
      "source": "thermal.frame",
      "score": 0.78,
      "relationship": "co-located_with_hotspot"
    }
  ],

  "importance_weights": {
    "school_count": 0.15,
    "thermal_break": 0.45,
    "depth_band": 0.10,
    "correlation_14:30:02Z": 0.30
  },

  "reasoning_steps": [
    {"step": 1, "claim": "8 school marks visible in 18-42 fm band",
     "evidence": "echogram.captures/2026-07-23/143218.png"},
    {"step": 2, "claim": "thermal_break=true with ΔT=1.8°C at 14:30",
     "evidence": "thermal.captures/2026-07-23/143002.json"},
    {"step": 3, "claim": "co-located with last week's hotspot at
                       47.6234°N, -122.3456°W",
     "evidence": "corpus.search(hotspot) -> 2026-07-19T13:48Z"}
  ],

  "alternatives": [
    {"hypothesis": "8 schools, all baitfish",
     "score": 0.21,
     "rejected_because": "thermal_break correlates with
                          feeding-size marks, not bait balls"},
    {"hypothesis": "mixed: 6 bait + 2 predator",
     "score": 0.62,
     "rejected_because": "no predator-arch marks in 18-42 fm band"}
  ],

  "conclusion": "8 schools; thermal break suggests active feeding;
                 re-score against new bait model.",

  "references": [
    "captures/2026-07-23/143218.png",
    "captures/2026-07-23/143002.json",
    "corpus/hotspots/2026-07-19.json"
  ],

  "annotations": [
    {
      "by": "casey",
      "at": "2026-07-23T15:01:00Z",
      "field": "conclusion",
      "from": "8 schools; thermal break suggests active feeding;",
      "to":   "8 schools; thermal break suggests active feeding;
              re-score against new bait model.",
      "reasoning": "added next-action so the analyzer queues
                   retraining on the next loop"
    }
  ]
}
```

This is **A2A-native**: another agent (or the analyzer, or the
projection) can read the file with no Markdown parsing, do
arithmetic on `embeddings` and `importance_weights`, follow
`references` to raw data, and audit `reasoning_steps` line by
line.

---

## The render: `MomentAnalysis` markdown

The same object renders to a markdown file like:

```markdown
# 2026-07-23 14:32:18 — Echogram analysis

*Produced by `granite4.1:8b@ollama` at 2026-07-23 14:35:02.
Confidence 0.82. Tokens 1840 in / 412 out. v3, 2 edits.*

## Headline

8 schools; thermal break suggests active feeding;
re-score against new bait model. *(edited by casey)*

## What I saw

- **8 schools**, sizes 4–22 fm, depth 18–42 fm (median 27).
- **Thermal break**: yes, ΔT 1.8 °C at 14:30.
- Bottom structure: shelf edge.

## Why I think so

1. 8 school marks visible in 18-42 fm band
   *(echogram/143218.png)*.
2. thermal_break=true with ΔT=1.8°C at 14:30
   *(thermal/143002.json)*.
3. co-located with last week's hotspot at
   47.6234°N, -122.3456°W
   *(corpus/hotspots/2026-07-19.json)*.

## What I rejected

- "8 schools, all baitfish" (0.21) — thermal break correlates
  with feeding-size marks, not bait balls.
- "mixed: 6 bait + 2 predator" (0.62) — no predator-arch marks
  in the band.

## Correlations

- **2026-07-23 14:30:02** (thermal.frame) — 0.78 —
  co-located with hotspot.

## See also

- echogram/143218.png
- thermal/143002.json
- corpus/hotspots/2026-07-19.json
```

The human reading this **in 8 seconds** knows:

- It's a real moment (not a model error).
- 8 schools, with a thermal break (the thing the captain cares about).
- The agent has reasoning they can audit if they want to.
- The agent already considered two alternatives and rejected them.
- The captain can scroll past if they have 30 of these to read.

---

## The round-trip: `f(JSON) = markdown`, `g(markdown) = JSONᵉ`

Two deterministic functions:

```python
# analysis/render.py
def render_markdown(analysis: dict) -> str:
    """Deterministic render: same JSON in, same markdown out.
    Pure function. No side effects."""
    ...

def parse_markdown_edits(markdown: str, original: dict) -> dict:
    """Parse a human-edited markdown back into an edit set.
    Returns a partial JSON patch, not a full JSON."""
    ...
```

**Render is a pure function.** Same JSON always produces the same
markdown. The agent never hand-writes markdown; the analyzer
emits JSON, the renderer emits markdown. If a render bug is
found, the bug is in `render.py`, and the fix re-renders all
existing analyses deterministically.

**Edit is structured, not freeform.** A human can edit the
markdown, but the editor is **structured**: clicking on the
"Headline" paragraph opens a form bound to `conclusion` in the
JSON. The "Why I think so" list items are bound to
`reasoning_steps[].claim` and `[].evidence`. The "What I
rejected" list is bound to `alternatives[].hypothesis` and
`[].rejected_because`. A human can also open the raw JSON
editor (a tab in the center pane) and edit any field directly.

Either way, the edit is:

1. Validated against the schema.
2. Appended to `analysis/{HHMMSS}.edits.jsonl` (append-only audit).
3. Applied to the JSON, bumping `version` and incrementing
   `edit_count`.
4. The markdown re-renders.
5. The moment is queued for the retraining loop.

---

## File layout (per moment)

Under `~/tzpro-personal/captures/{date}/analysis/`:

```
{HHMMSS}.json           # canonical truth (the MomentAnalysis)
{HHMMSS}.md             # deterministic render of the JSON
{HHMMSS}.v1.json        # snapshot of version 1 (if v>1)
{HHMMSS}.v2.json        # snapshot of version 2
{HHMMSS}.edits.jsonl    # append-only audit log of edits
{HHMMSS}.embeddings.npy # numpy array of embeddings (when vectors grow)
```

The `.v{n}.json` snapshots are **discoverable archaeology** — the
human can diff v1 vs v3 to see what changed. The `.edits.jsonl`
is **the lineage** — who changed what, when, why. The `.json` is
**the truth** — current state.

The `.edits.jsonl` is also the **retraining dataset** — every
edit is a labeled example of "the captain disagreed with the
agent here, here's the better answer." This is the LoRA flywheel
at the analysis layer, not the model layer.

---

## The projection's view: three tabs, click-to-highlight

The center pane of the projection renders the moment in three
toggleable views (see `architecture/PROJECTION_LAYER.md` for
the full panel model):

| View | Source | Renders |
|---|---|---|
| **Image** | `captures/{date}/{HHMMSS}.{ext}` | the raw echogram, thermal frame, audio spectrogram |
| **JSON** | `analysis/{HHMMSS}.json` | a tree with field-level highlighting |
| **Markdown** | `analysis/{HHMMSS}.md` | the pre-rendered skim version |

**Click-to-highlight** is the synoptic primitive. When the human
clicks a feature in one view, the other two highlight the
corresponding field/anchor:

- Click a school in the **image** → JSON highlights
  `features.school_sizes_fm[index]` and `importance_weights.school_count`;
  markdown scrolls to the bullet.
- Click `confidence` in **JSON** → markdown highlights the
  "Produced by …" subhead; image overlays the school's
  bounding box (if present in the analysis's `references`).
- Click a `correlations[].with_moment` in **JSON** → markdown
  jumps to the "Correlations" section; the linked moment is
  preloaded in the left navigator as "next."

The click is **read-only** by default. The human reads with
their eyes first; when they decide to edit, they switch the
view into edit mode and the structured form appears.

---

## Edit primitive: form for the captain, JSON for the engineer

Two editing surfaces for the same underlying edit:

1. **Structured form** (default). The view switches from
   "render" mode to "edit" mode. Field-level inputs appear:
   - Headline: a text area bound to `conclusion`.
   - Reasoning steps: an ordered list, each step with
     `claim` and `evidence` fields.
   - Alternatives: a list, each with `hypothesis`, `score`,
     `rejected_because`.
   - Annotations: per-field notes (`by`, `at`, `reasoning`).
   The human edits the form; the JSON diff is computed;
   `version` bumps; markdown re-renders.

2. **Raw JSON editor** (engineer mode). A JSON-aware text
   editor (Monaco or CodeMirror) opens the full `MomentAnalysis`.
   Edits are validated against the schema on save; the same
   diff/commit/render path runs.

Either way, the **edit primitive is the same**: a structured
patch against the JSON, audited, versioned, regenerated, queued
for retraining. The shape of the input doesn't matter; the
shape of the output does.

---

## What "round-trip" means in practice

A clean round-trip is:

```
JSON ---render_markdown()---> markdown
markdown ---parse_markdown_edits()---> edit_set
edit_set ---apply_to_json()---> JSON'
JSON' ---render_markdown()---> markdown'
```

`markdown == markdown'` after a no-op edit, and the human sees
no flicker. With actual edits, `markdown'` reflects the
changes. The render is **idempotent** and **pure** — no hidden
state, no time-based output, no model calls.

The render is **content-stable across versions** — adding a new
field to the schema doesn't break old markdown; it just renders
the new field when present and skips it when absent. Backward
compatibility is a property of the renderer, not a constant
firefighting job.

---

## The retraining loop (the LoRA flywheel at the analysis layer)

Every edit, by the captain or by another agent, lands in
`{HHMMSS}.edits.jsonl` with:

- `by` (author: `casey`, `granite4.1:8b`, etc.)
- `at` (timestamp)
- `field` (path within the JSON)
- `from` (previous value)
- `to` (new value)
- `reasoning` (free text)

A nightly job (or on-demand) reads the edit log and:

1. **Diffs** the JSON to extract the agent's "before" answer and
   the human's "after" answer.
2. **Pairs** them with the raw capture (image, audio,
   telemetry) that the agent was analyzing.
3. **Formats** each pair as a LoRA training example
   (input: capture + analysis prompt; output: corrected
   analysis).
4. **Appends** to `corpus/training/{date}.jsonl`.
5. **Triggers** a fine-tune job (cloud, when budget allows) on
   the local model (e.g. `granite4.1:8b` → `boat-agent-v0.1`).

This is the same flywheel as the global LoRA (mother-agent →
training data → better local model → less cloud) but at the
**analysis layer**: the captain is teaching the local model how
to analyze *their* fishing, on *their* boat, with *their*
priorities. The flywheel compounds.

---

## Phasing

| Phase | What lands |
|---|---|
| **Phase 2** | The `MomentAnalysis` schema and the analyzer that emits it. JSON only at first; markdown is a debug print. |
| **Phase 2.5** | The `render.py` and `parse_markdown_edits()` round-trip. Markdown becomes a first-class artifact. |
| **Phase 10** | The projection renders the three views with click-to-highlight and the structured form. |
| **Phase 10.1** | Edit primitive ships; `edits.jsonl` starts filling. |
| **Phase 8 (cloud)** | The edits and snapshots sync to R2/D1; the corpus is shared across devices and across time. |
| **Phase ∞** | The flywheel produces a fine-tuned local model that the captain's edits effectively programmed. |

---

## See also

- `docs/architecture/PROJECTION_LAYER.md` — the runtime that
  toggles between the three views and runs the edit primitive.
- `docs/architecture/CAPTURE_PIPELINE.md` — the `Moment` schema
  and how `analysis` hangs off it.
- `docs/AGENT_OPERATING_MODEL.md` — the IO contract (JSON is
  a2a-native, markdown is human).
- `docs/SUIT_VS_PERSON.md` — the privacy boundary; analysis
  JSON lives in `~/tzpro-personal/`, not the suit.
