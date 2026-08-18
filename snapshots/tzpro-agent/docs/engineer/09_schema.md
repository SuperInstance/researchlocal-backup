# Schema Layer — `schema/`

**Package:** `schema/`
**Priority:** Foundation; everything downstream of capture reads from this.
**Owner:** tzpro-agent.

## What It Does

Defines the **atomic units of data** that flow through the agent.
Every sensor (sounder screenshot, NMEA fix, engine telemetry, voice
note, analyzer output) is wrapped in a Moment; downstream code
queries the stream without caring about the source.

The design principle, stated in the file's own header:

> **Schema is stable, payload is open.** Every Moment has the same
> envelope (timestamp + position + source + payload). The payload
> itself is a free-form dict whose keys are per-source. That way
> new sensors can be added without schema migrations.

> **JSON truth, markdown render.** Every Moment serializes
> deterministically via `to_dict()`. The companion `from_dict()`
> parses. `markdown_render()` is a *separate* function that produces
> human-readable text — it's `f(JSON)`, never the source of truth.

## Files

| File | Purpose |
|---|---|
| `schema/__init__.py` | Re-exports the public API |
| `schema/moment.py` | `Moment` + `Position` dataclasses |
| `schema/anomaly.py` | Anomaly report shape |
| `schema/correlation.py` | Cross-source correlation record |
| `schema/vessel.schema.json` | JSON Schema for the vessel record (vessel.json) |

## `Position` (`schema/moment.py:42-54`)

```python
@dataclass
class Position:
    lat: Optional[float] = None
    lon: Optional[float] = None
    sog_knots: Optional[float] = None
    cog_deg: Optional[float] = None
    fix_quality: str = "unknown"  # "gps" | "dgps" | "estimated" | "none" | "unknown"
```

Position is **nullable, not zero-defaulted** — zero is a real place
(Gulf of Guinea). This is a deliberate doctrine; do not "fix" it.

## `Moment` (`schema/moment.py:57-…`)

```python
@dataclass
class Moment:
    id: str                        # m_YYYYMMDDTHHMMSS_<8-hex>
    timestamp: str                 # ISO-8601 UTC, second precision
    source: str                    # "sounder" | "nmea" | "engine" | "thermal" |
                                   # "audio" | "voice" | "analysis"
    position: Position             # boat position at capture time
    payload: dict                  # free-form, per-source schema
    tags: list[str] = []           # free-form labels
    importance: float = 0.0        # 0.0–1.0, set by source or analyzer
    parent_id: Optional[str] = None  # link analysis → its inputs
```

### Conventions for `source`

| Source | `payload` keys |
|---|---|
| `sounder` | `{path, width, height, hash}` |
| `nmea` | `{sentence, parsed}` |
| `engine` | `{pgn, fields}` |
| `thermal` | `{path, min_c, max_c, mean_c}` |
| `audio` | `{path, rms, peak, centroid_hz}` |
| `voice` | `{text, stt_model, duration_s}` |
| `analysis` | `{model, prompt, response, confidence, inputs}` |

### Conventions for `importance`

- Anomalies start at 0.8+.
- Routine telemetry at 0.1–0.3.
- The analyzer may raise importance based on content.

### `parent_id` for Analyses

If a `Moment` is the result of analyzing other Moments, `parent_id`
points to the primary input. For an analyzer that ingests 10 captures,
it typically emits 10 Moments each with `parent_id = capture.id`.

## Serialization

```python
m = Moment(source="sounder",
           position=Position(lat=55.79, lon=-131.66),
           payload={"path": "captures/v3/2026-07-23_5547N_13141W/0840.png",
                    "width": 1920, "height": 1080, "hash": "..."})
d = m.to_dict()                  # dict (JSON-safe)
j = m.to_json()                  # str (canonical JSON)
m2 = Moment.from_json(j)         # round-trip
md = m.markdown_render()         # f(d) for humans
```

`to_dict()` produces stable, JSON-safe output suitable for append-only
logs. The `payload` dict is preserved as-is.

`from_dict()` / `from_json()` parse a dict/string back into a Moment,
tolerating missing fields with defaults.

`markdown_render()` produces a human-readable summary. **Never** read
from the markdown to make decisions; always go through `to_dict()`
or the original capture.

## `Anomaly` (`schema/anomaly.py`)

A second-order shape: describes an unusual pattern detected across
one or more Moments. Captures: rule that fired, score, time window,
linked Moments.

## `Correlation` (`schema/correlation.py`)

A third-order shape: cross-source patterns. E.g. "wind shift at 14:32
correlates with bite drop at 14:40 across 8 days."

## `vessel.schema.json` (`schema/vessel.schema.json`)

JSON Schema (Draft 7) for the `vessel.json` config file. Editors
with JSON Schema support can validate the config in real time.

## Adding a New Source Type

1. Decide on `source` string (kebab-case or snake_case — be consistent).
2. Decide on `payload` keys (document here).
3. Update `docs/architecture/CAPTURE_PIPELINE.md` to mention it.
4. Optionally update `schema/vessel.schema.json` if it affects routing.

No code change required to `Moment` itself — the payload dict
absorbs new keys.

## Verified Line Numbers (as of `b64c2ef`)

- `_utcnow_iso`, `_new_id`: `schema/moment.py:31-39`
- `Position`: `schema/moment.py:42-54`
- `Moment`: `schema/moment.py:57-…`
- `to_dict` / `to_json` / `from_dict` / `from_json` / `markdown_render`:
  search `def to_`, `def from_`, `def markdown_` in `schema/moment.py`

## Related Docs

- `docs/architecture/CAPTURE_PIPELINE.md` — multi-source vision
- `docs/architecture/DUAL_REPRESENTATION.md` — JSON-truth principle
- `docs/SUIT_VS_PERSON.md` — Moment payload is "suit" data