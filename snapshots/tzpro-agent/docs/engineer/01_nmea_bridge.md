# NMEA Bridge — `nmea_bridge.py`

**File:** `nmea_bridge.py` (~1100 lines)
**Priority:** P0 — without this, TZ Pro has no position.
**Owner:** tzpro-agent / `boat/runbook.md` references this for restart.

## What It Does

Reads NMEA 0183 sentences from a u-blox GPS receiver on `COM6 @ 4800 baud`,
parses them into a live `VesselState` dataclass, and fans them out to:

1. **TCP `:6006`** — raw NMEA verbatim, for TimeZero Professional.
2. **HTTP `:8654`** — JSON API for the dashboard, doctor, and analyst tools.
3. **`vessel_state.jsonl`** — append-only JSONL log; the "first-class-citizen
   database" that downstream code reads.
4. **`.last_nmea_heartbeat`** — timestamp file used by `doctor.py` to detect
   silent bridge death.

The bridge is **read-only**. We never write to COM6; TZ Pro owns the port
exclusively. We coexist by opening with `FILE_SHARE_READ | FILE_SHARE_WRITE`
(`nmea_bridge.py:218-220`).

## The Win32 Shared-Mode Trick

`pyserial`'s default `Serial(...)` opens the COM port with
`share_mode=0`, which fails with `PermissionError` if any other process
has the port open (TZ Pro does). The bridge bypasses this by calling
`kernel32.CreateFileW` directly via `ctypes` with the share flags set
(`nmea_bridge.py:250-258`):

```python
handle = kernel32.CreateFileW(
    port,
    GENERIC_READ | GENERIC_WRITE,
    FILE_SHARE_READ | FILE_SHARE_WRITE,  # <- the magic
    None, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, None,
)
```

If even this fails (some TZ Pro builds have signed-driver protection that
blocks *any* second reader), the bridge falls back to a `com0com` virtual
pair — typically reading from `COM12` while TZ Pro writes to `COM11`.
The fallback list is `COM0COM_FALLBACKS` (`nmea_bridge.py:123`).

## Process & Thread Topology

```
┌──────────────────────────────────────────────────────────┐
│  asyncio loop (main thread)                              │
│                                                          │
│  ┌─ TcpBroadcaster (asyncio)         ──> :6006 clients   │
│  ├─ JsonlWriter (asyncio)            ──> vessel_state.jsonl
│  ├─ SseBroadcaster (asyncio)         ──> /stream clients │
│  ├─ aiohttp app (asyncio)            ──> :8654 routes    │
│  └─ heartbeat_loop (asyncio)         ──> .last_nmea_heartbeat
│                                                          │
│  All consume from state_q (asyncio.Queue)                │
└──────────────────────┬───────────────────────────────────┘
                       │
┌──────────────────────┴───────────────────────────────────┐
│  SerialReaderThread (threading.Thread, daemon=True)      │
│                                                          │
│  • Blocking ReadFile() loop on the Win32 handle          │
│  • Parses each sentence into a state delta               │
│  • Pushes (state_snapshot, raw_sentence) onto state_q    │
│    via loop.call_soon_threadsafe                          │
└──────────────────────────────────────────────────────────┘
```

The split exists because `ReadFile()` on a Win32 handle is a blocking
syscall; doing it on the asyncio loop would freeze the HTTP server.
The state queue is the synchronization boundary.

## `VesselState` — The Boat as a Dataclass

`nmea_bridge.py:145-208`. One dataclass, updated in place as sentences
arrive. Fields default to `None` = "not yet known"; consumers must
tolerate `None`.

| Group | Fields |
|---|---|
| Time | `timestamp_utc`, `timestamp_local` |
| Position | `lat`, `lon`, `fix_quality`, `satellites`, `hdop`, `altitude_m` |
| Motion | `sog_kts`, `cog_deg`, `mag_variation` |
| Heading | `heading_true_deg`, `heading_mag_deg` |
| Depth | `depth_m`, `depth_ft`, `depth_fm` |
| Wind | `wind_speed_kts`, `wind_dir_deg` |
| Computed | `state_class` (docked / trolling / cruising) |
| Bookkeeping | `source_port`, `sentence_count`, `last_sentence_id` |

`as_dict()` drops `None` values (`nmea_bridge.py:183-186`) — keeps
JSONL compact and queries fast.

`classify_motion()` (`nmea_bridge.py:198-208`):

| SOG (kts) | state_class |
|---|---|
| < 0.5 | `docked` |
| 0.5 – 2.5 | `trolling` |
| 2.5 – 8.0 | `slow_cruise` |
| ≥ 8.0 | `cruising` |
| unknown | `unknown` |

This classification drives dashboard badges and downstream alerts.

## NMEA Sentence Coverage

| Sentence | Source fields |
|---|---|
| `$GPGGA` | lat, lon, fix quality, satellites, HDOP, altitude, UTC time |
| `$GPRMC` | lat, lon, SOG, COG, magnetic variation, date+time |
| `$GPGLL` | lat, lon (backup) |
| `$GPHDT` | heading true (some pilots) |
| `$HCHDT` | heading true (Airmar / Simrad default) |
| `$HCHDG` | heading magnetic + deviation → derived true |
| `$SDDBT` | depth ft/m/fm (whichever present, derived for the rest) |
| `$SDDPT` | depth m (Simrad default) |
| `$VHW` | heading true/mag (backup) |
| `$MWV` | wind speed/dir (logged, not yet fused) |

Each sentence is checksum-verified (`_verify_checksum`,
`nmea_bridge.py:389-406`) before parsing. Bad checksums are silently
dropped (logged at debug level).

## HTTP Routes (`:8654`)

| Route | Method | Response | Used by |
|---|---|---|---|
| `/health` | GET | `{"ok": true, ...}` | doctor liveness |
| `/ready` | GET | `{"ready": true, "fix_quality": 1, "satellites": 9, ...}` | doctor readiness, tray badge |
| `/vessel` | GET | full `VesselState` snapshot | dashboard, analyst |
| `/vessel/history?limit=N` | GET | last N snapshots from JSONL | analyst queries |
| `/stream` | GET (SSE) | newline-delimited JSON events | dashboard live view |
| `/nmea/raw` | GET | last 200 raw sentences (ring buffer) | debugging |

The `/ready` endpoint is the truth-or-lie signal for the system:
`ready: true` only when `serial_open && fix_acquired`.

## TCP Broadcast (`:6006`)

`TcpBroadcaster` (`nmea_bridge.py`, class definition ~line 760):

- Accepts up to 8 simultaneous clients (more than TZ Pro + capture_v3).
- Multiplexes: every connected client gets every sentence.
- No request/response — pure firehose, just like the GPS is broadcasting.
- If a client slows down, the broadcaster drops its connection (we don't
  buffer for slow consumers; TZ Pro never slows down).

## JSONL Writer

`JsonlWriter` writes one line per state delta to `vessel_state.jsonl`:

```json
{"timestamp_utc":"2026-07-24T16:42:01.123456+00:00","timestamp_local":"2026-07-24T08:42:01.123456-08:00","lat":55.79178,"lon":-131.65511,"fix_quality":1,"satellites":8,"hdop":1.2,"sog_kts":2.05,"cog_deg":329.7,"state_class":"trolling","last_sentence_id":"GPRMC","sentence_count":1117,"source_port":"COM6"}
```

- Append-only; rotated by the operator manually when file exceeds ~1 GB.
- Each line is a valid JSON object.
- Consumers (dashboard, analyst) tail this file and re-derive state.
- The file is `.gitignore`d (`vessel_state.jsonl` is too valuable to
  risk in source control).

## Heartbeat

Every 5 s the asyncio loop writes `time.time()` to
`.last_nmea_heartbeat` (`nmea_bridge.py:127` →
`HEARTBEAT_INTERVAL_S`, function ~line 1080).
`doctor.py` checks this file; if it is >15 s stale, the bridge is
considered dead and `fix --yes` is supposed to restart it.

## Restart Pattern

```powershell
# From C:\Users\casey\tzpro-agent\
powershell -Command "Get-Process python -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like '*nmea_bridge*' } | Stop-Process -Force"
Start-Process pythonw -ArgumentList "nmea_bridge.py --port COM6 --baud 4800" -WorkingDirectory "C:\Users\casey\tzpro-agent\"
```

Or use the helper:

```bash
python fix_priority0.bat
```

(`fix_priority0.bat` lives at repo root and kills + restarts all P0 services.)

## Diagnostic Mode

```bash
python nmea_bridge.py --diag --diag-seconds 8
```

Dumps raw NMEA sentences to stdout for 8 s, parses them, prints
parsed state, exits. **Always run this first when chasing a "no GPS"
issue.** If you see sentences but TZ Pro shows no position, the bug
is downstream; if you see no sentences, the bug is upstream (cable,
GPS power, wrong COM port).

## Fallback: com0com Pair

If the shared-mode open fails repeatedly, install com0com and create
a virtual pair `COM11 ↔ COM12`. Configure TZ Pro to read from `COM11`,
configure the bridge to read from `COM12`:

```bash
python nmea_bridge.py --port COM12 --baud 4800
```

The fallback chain is `COM12, COM10, COM11, COM9` (`nmea_bridge.py:123`).
Edit the list if your pair has different numbers.

## Common Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| `CreateFileW failed (Win32 error 5)` | TZ Pro has port exclusive-locked, signed-driver block | Install com0com pair, use fallback port |
| No sentences but bridge alive | GPS not powered / wrong baud / wrong COM port | Check power, try `--baud 9600` or `--port COM7` |
| Sentences arrive but `lat` is None | Sentence format variant we don't parse | Add case to `parse_sentence` |
| `vessel_state.jsonl` not growing | JsonlWriter task died | Restart bridge; if recurrent, file a bug |
| TZ Pro connected but no position | TZ Pro's NMEA source config wrong | In TZ Pro: `Initialization → GPS → Port=TCP 127.0.0.1:6006, Protocol=NMEA0183` |

## Verified Line Numbers (as of `b64c2ef`)

- Win32 constants: `nmea_bridge.py:216-224`
- `open_shared_serial`: `nmea_bridge.py:234-332`
- `_nmea_latlon`: `nmea_bridge.py:346-368`
- `_verify_checksum`: `nmea_bridge.py:389-406`
- `parse_sentence`: `nmea_bridge.py:409-578`
- `VesselState.classify_motion`: `nmea_bridge.py:198-208`
- Routes (`/health`, `/ready`, etc.): search `nmea_bridge.py` for
  `async def health`, `async def ready`, etc.

## Related Docs

- `docs/HARDWARE_SETUP.md` — physical wiring
- `docs/TROUBLESHOOTING.md` — symptom → fix cookbook (legacy; some
  entries still useful)
- `docs/architecture/CAPTURE_PIPELINE.md` — multi-modal vision of capture
- `docs/engineer/02_capture_daemon.md` — TZ Pro lifecycle supervisor
  (the bridge is its upstream)