# Capture Worker — `capture_v3.py`

**File:** `capture_v3.py` (~370 lines)
**Priority:** P1 — produces the echogram data that is the whole reason
this system exists.
**Owner:** tzpro-agent.
**Run by:** `capture_daemon.py` (or `--oneshot` for tests).

## What It Does

Every 10 minutes on the hour boundary (00, 10, 20, 30, 40, 50):

1. Reads the boat's current position from `nmea_bridge` over TCP :6006
   (raw NMEA).
2. Spawns `screenshot_v3.ps1` to grab DISPLAY6 (the second monitor at
   X=1920, 1920×1080) — this is the echogram.
3. Writes three files together (atomic write via `.tmp` → rename):
   - `<HHMM>_<lat>N_<lon>W.png` — the image
   - `<HHMM>_<lat>N_<lon>W.json` — A2A-native metadata
   - `<HHMM>_<lat>N_<lon>W.md` — human-readable annotation
4. POSTs a concise summary to **Ship Log Search** (Cloudflare Worker,
   fire-and-forget).

## Cadence Math

`wait_for_next_boundary()` (`capture_v3.py:324-334`):

```python
block_min = (now.minute // 10) * 10
next_b = (block_min + 10) - now   # seconds until next 10-min boundary
```

Aligning to boundaries (instead of `time.sleep(600)` from start) means
all captures across multiple vessels line up at the same wall-clock
minute — useful for fleet-scale analysis later.

## Day Folder Naming

Folders are named `<YYYY-MM-DD>_<start_lat>N_<start_lon>W` using the
**first position of the day** (`capture_v3.py:172-179`). This makes
the folder name stable for the whole day even as the boat drifts.

```
captures/v3/
└── 2026-07-23_5547N_13141W/
    ├── 0840_5547.643N_13140.201W.png
    ├── 0840_5547.643N_13140.201W.json
    ├── 0840_5547.643N_13140.201W.md
    ├── 0850_5547.687N_13140.698W.png
    ├── 0850_5547.687N_13140.698W.md
    └── …
```

The lat/lon in the **filename** updates every capture (so you can find
the boat's position from the filename alone); the lat/lon in the
**folder name** stays fixed for the day.

## The Three Files (Twin Format)

For every capture we write three files representing the same
information in three languages:

### `.png` — the image

DISPLAY6 screenshot, 1920×1080 PNG. This is the echogram — the
primary data asset. Disk: ~50–200 KB each.

### `.json` — A2A twin (machine-to-machine)

`capture_v3.py:200-219` builds this dict:

```json
{
  "capture_id": "0840_5547.643N_13140.201W",
  "ts_utc": "2026-07-24T16:40:00.123456+00:00",
  "ts_local": "2026-07-24T08:40:00.123456-08:00",
  "ts_local_hhmm": "0840",
  "frame_file": "0840_5547.643N_13140.201W.png",
  "position": {
    "lat_dd": 55.79405,
    "lon_dd": -131.67002,
    "lat_ddmm": "5547.643",
    "lon_ddmm": "13140.201",
    "sog_kts": 2.05,
    "cog_deg": 329.7
  },
  "display": {
    "offset_x": 1920,
    "offset_y": 0,
    "width": 1920,
    "height": 1080,
    "depth_max_fm": 60,
    "px_per_fm": 18.0
  },
  "analysis": {
    "schema_version": 1,
    "heuristic": null,
    "caption": null,
    "vocabulary": null
  },
  "edges": {
    "neighbors_time": [],
    "neighbors_space": []
  }
}
```

The `analysis` block is intentionally empty — downstream analyzers
(hermes, sounder_analyzer) fill it in post-hoc. `edges` will eventually
link captures by temporal and spatial proximity (k-NN search).

### `.md` — human twin

`capture_v3.py:228-260` writes a markdown header + body:

```markdown
# Echogram Capture  0840_5547.643N_13140.201W

**Date:** July 24, 2026  **Time:** 08:40 AKDT

## Vessel
- Position: 5547.643N  13140.201W  (DDMM.mmm)
- SOG 2.05 kn  COG 330deg

## Display
- Monitor: DISPLAY6 (1920x1080 @ X=1920)
- Mode: Dual-band sounder, fixed 60 fm range
- Scale: 18.0 px/fathom

## Water Column (0-60 fm)
Surface:     0-5 fm   — clutter zone
Upper:      5-20 fm   — bait, pelagics
Mid:       20-40 fm   — target depth zone (chum)
Lower:     40-55 fm   — near-deep
Floor:     55-60 fm   — display limit

## Analysis
*Raw capture — no analysis yet.*

---
*capture_v3.py at 08:40:00 AKDT*
```

This file is the "first thing the captain reads at 8 AM". It is the
`f(JSON)` of the capture — same truth, different render.

> **Doctrine:** JSON is truth; markdown is `f(JSON)`. If they disagree,
> fix the JSON, then re-derive the markdown.

## Atomic Writes

Both `.json` and `.md` are written via `.tmp` → `Path.replace()`
(`capture_v3.py:222-226, 256-260`). This guarantees that consumers
(tail-following analysts) never see a half-written file, even if the
process crashes mid-write.

The PNG itself is written directly by `screenshot_v3.ps1`; if it
crashes the PNG may exist without its twin files. The daemon's
`verify` subcommand notices this and reports STALE.

## Ship Log Search Ingest

`ship_log_ingest()` (`capture_v3.py:273-321`):

After every successful capture, POST a summary to
`https://ship-log-search.casey-digennaro.workers.dev/api/log`.

Payload is a single text blob suitable for embedding + semantic search:

```json
{
  "text": "Echogram Capture … Vessel Position: … Display …",
  "category": "observation",
  "subcategory": "echogram_capture",
  "timestamp": "2026-07-24T16:40:00.123456+00:00",
  "lat": 55.79405,
  "lon": -131.67002,
  "location_name": "5547.643N/13140.201W",
  "id": "echogram_0840_5547.643N_13140.201W",
  "metadata": { … }
}
```

- `User-Agent` is faked to look like a browser (the Worker requires it).
- `timeout=5 s` — never blocks capture.
- Failures are logged at warning level and discarded. The cloud index
  is a "nice to have"; local captures are the truth.

## NMEA Position Read

`fetch_position()` (`capture_v3.py:97-136`) opens a fresh TCP socket to
`127.0.0.1:6006` (the bridge), reads up to ~2 KB of NMEA, parses GPGGA
and GPRMC, returns `(lat, lon, sog, cog)`.

- Timeout 5 s (`NMEA_TIMEOUT_S`, `capture_v3.py:42`).
- If bridge is down: returns `None` and the capture proceeds with
  placeholder `0000.000N 00000.000W` position. This is intentional —
  we never want to skip a capture just because GPS hiccupped.

## PowerShell Screenshot

`ensure_script()` (`capture_v3.py:58-74`) writes `screenshot_v3.ps1`
on first run if missing. The script uses `System.Drawing` to grab
the second monitor:

```powershell
$bmp = New-Object System.Drawing.Bitmap(1920, 1080)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen(1920, 0, 0, 0, [System.Drawing.Size]::new(1920, 1080))
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
```

- Works on Windows 10/11 with a second monitor at X=1920.
- **No headless support** — requires a logged-in desktop session.
- `screenshot_v3.ps1` is regenerated by `ensure_script()` if missing,
  so the worker is self-installing.

## Tuning Knobs

| Constant | Default | Effect |
|---|---|---|
| `CAPTURE_INTERVAL_MIN` | 10 | Minutes between captures |
| `DISPLAY_OFFSET_X` | 1920 | X coordinate of second monitor |
| `DISPLAY_WIDTH/HEIGHT` | 1920×1080 | Capture resolution |
| `DEPTH_MAX_FM` | 60 | Sounder range; px/fm derived |
| `NMEA_TIMEOUT_S` | 5 | Bridge read timeout |

To change cadence, edit `CAPTURE_INTERVAL_MIN` here **and** import it
from `capture_daemon.py` (which already does
`DEFAULT_CAPTURE_INTERVAL_MIN = 10`, `capture_daemon.py:70`) so the
verdict threshold stays consistent.

## Verified Line Numbers (as of `b64c2ef`)

- Constants: `capture_v3.py:30-48`
- NMEA position fetch: `capture_v3.py:97-136`
- Day folder logic: `capture_v3.py:139-153, 172-179`
- `capture_frame` body: `capture_v3.py:156-270`
- Ship Log ingest: `capture_v3.py:273-321`
- Boundary cadence: `capture_v3.py:324-334`
- Main entrypoint: `capture_v3.py:349-368`

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `No valid file produced` | `screenshot_v3.ps1` failed (no second monitor / no console session) | Plug in second monitor; check `DISPLAY_OFFSET_X` |
| Ship Log ingest fails every time | Worker endpoint moved or expired | Update `SHIP_LOG_URL` (or accept the warnings) |
| Captures drift off-boundary | `time.sleep` drift accumulates | `wait_for_next_boundary()` re-aligns each tick |
| PNG present, no JSON | Process killed mid-write | Run `verify`; manually delete orphan PNGs or accept the gap |
| All captures show 0000.000N | Bridge down | Restart bridge; see `01_nmea_bridge.md` |

## Related Docs

- `docs/engineer/01_nmea_bridge.md` — upstream of position data
- `docs/engineer/02_capture_daemon.md` — supervisor that owns this process
- `docs/architecture/CAPTURE_PIPELINE.md` — multi-modal vision of capture
- `docs/architecture/DUAL_REPRESENTATION.md` — JSON/MD twin principle