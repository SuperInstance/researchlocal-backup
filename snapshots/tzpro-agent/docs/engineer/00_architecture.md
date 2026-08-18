# Architecture Overview

## Bird's-Eye View

```
                           ┌─────────────────────────────────────┐
                           │   u-blox GPS Receiver (NMEA 0183)   │
                           │   physical: USB → COM6 @ 4800 baud  │
                           └─────────────┬───────────────────────┘
                                         │ serial
                                         ▼
   ┌─────────────────────────────────────────────────────────────┐
   │  nmea_bridge.py                                              │
   │  • ctypes Win32, FILE_SHARE_READ|WRITE                       │
   │  • parses NMEA into Position struct                           │
   │  • serves raw NMEA TCP :6006 (TZ Pro consumer)                │
   │  • serves JSON HTTP :8654 (/vessel, /stream, /health, /ready)│
   │  • appends vessel_state.jsonl                                │
   └────┬──────────────────────────────────────┬─────────────────┘
        │ TCP :6006 raw NMEA                    │ HTTP :8654 JSON
        ▼                                       ▼
   ┌──────────────────────┐             ┌──────────────────────┐
   │  TimeZero Pro         │             │  dashboard.py        │
   │  (TimeZero.exe)       │             │  Starlette :8090     │
   │  shows ship on chart  │             │  LAN-reachable UI    │
   └──────────────────────┘             └──────────┬───────────┘
                                                    │
   ┌────────────────────────────────────────────────┘
   │  pystray tray_app.py
   │  • menu: Dashboard / Folders / Start/Stop / Verify / Status
   │  • auto-starts capture_daemon.py on launch
   │  • reads stampfile for freshness
   │
   ▼
   ┌──────────────────────────────────────────────────────────────┐
   │  capture_daemon.py — supervisor                                │
   │  • polls every 5s for TimeZero.exe                             │
   │  • launches capture_v3.py on first sighting                   │
   │  • stops capture_v3 30s after TZ Pro exit                     │
   │  • maintains stampfile (PID, child PID, mode, last_capture_at) │
   │  • subcommands: run / once / stop / status / doctor / verify  │
   └────┬─────────────────────────────────────────────────────────┘
        │ subprocess
        ▼
   ┌──────────────────────────────────────────────────────────────┐
   │  capture_v3.py — capture worker                                │
   │  • sleeps until next 10-min boundary                           │
   │  • reads position from bridge :6006                            │
   │  • invokes screenshot_v3.ps1 → DISPLAY6 (1920×1080 @ X=1920)  │
   │  • writes PNG + JSON (A2A twin) + MD (human twin)             │
   │  • atomic write via .tmp → rename                              │
   │  • POSTs summary to Ship Log Search (fire-and-forget)          │
   └────┬─────────────────────────────────────────────────────────┘
        │ HTTPS POST
        ▼
   ┌──────────────────────────────────────────────────────────────┐
   │  Ship Log Search (Cloudflare Worker)                           │
   │  casey-digennaro.workers.dev                                   │
   └──────────────────────────────────────────────────────────────┘
```

## Process Topology (Typical)

| Process | Started by | Lifetime |
|---|---|---|
| `TimeZero.exe` | Captain | Manual |
| `nmea_bridge.py` | `fix_priority0.bat` / tray "Restart bridge" | Long-lived (days) |
| `dashboard.py` | `fix_priority0.bat` | Long-lived |
| `tray_app.py` | `TZ Pro Agent Tray.lnk` | Captain's session |
| `capture_daemon.py` | tray auto-start / manual | Long-lived |
| `capture_v3.py` | daemon (subprocess) | Lives only while TZ Pro is up |

## Files of Record

| Path | Writer | Reader | Purpose |
|---|---|---|---|
| `bridge/state.jsonl` | nmea_bridge | doctor | Bridge health events |
| `vessel_state.jsonl` | nmea_bridge | dashboard / analyst | Per-fix vessel snapshot |
| `capture_daemon.stamp.json` | capture_daemon | tray / doctor | Daemon PID, child PID, freshness |
| `captures/v3/<day>/<HHMM>_*.png` | capture_v3 | analyst / dashboard | Echogram image |
| `captures/v3/<day>/<HHMM>_*.json` | capture_v3 | analyst / cascade | A2A-native metadata |
| `captures/v3/<day>/<HHMM>_*.md` | capture_v3 | humans | Human-readable annotation |
| `.alert_state.json` | alerts | doctor | Active alerts |
| `.tide_pool.json` | tide_pool | doctor | Pending async tasks |

## Network Topology

| Port | Protocol | Owner | Consumer |
|---|---|---|---|
| COM6 | serial | u-blox GPS | nmea_bridge |
| TCP :6006 | raw NMEA | nmea_bridge | TimeZero Professional |
| HTTP :8654 | JSON | nmea_bridge | dashboard, doctor |
| HTTP :8090 | Starlette | dashboard.py | captain's browser (LAN) |

> **Why two ports?** TZ Pro requires a raw NMEA stream — it does not speak
> JSON. Our other tools (dashboard, doctor, future analyst) want structured
> JSON. So the bridge serves both: raw for legacy, structured for everything
> new. Do not collapse them.

## State Machine — Capture Lifecycle

```
                  ┌────────────────────┐
                  │  TZ Pro not seen   │
                  │  (capture_daemon   │
                  │   polls every 5s)  │
                  └──────────┬─────────┘
                             │ TimeZero.exe seen
                             ▼
                  ┌────────────────────┐
                  │   TZ Pro running   │
                  │  capture_v3 spawns │
                  │  (every 10 min)    │
                  └──────────┬─────────┘
                             │ TimeZero.exe gone ≥ 30s
                             ▼
                  ┌────────────────────┐
                  │  capture_v3 stopped│
                  │  daemon waits      │
                  └────────────────────┘
```

## Boot Sequence (Reference Order)

1. **Windows boots** — no agent runs yet.
2. **Captain launches TZ Pro** — `TimeZero.exe` opens, sees no GPS (because bridge is down).
3. **Captain double-clicks `TZ Pro Agent Tray.lnk`** —
   - `tray_app.py` runs.
   - `_ensure_daemon_running()` calls `capture_daemon.py run --auto` if not already up.
   - `tray_app.py` probes bridge on :8654; if down, prompts to restart.
4. **Bridge starts** (via `fix_priority0.bat` or tray menu) —
   - `nmea_bridge.py` opens COM6 in shared mode.
   - Begin relaying NMEA on :6006.
5. **TZ Pro sees GPS** — Captain's chart now shows vessel position.
6. **Daemon sees TZ Pro** — spawns `capture_v3.py`.
7. **`capture_v3.py` enters run loop** — captures every 10 min on boundary.

## Failure Modes & Defenses

| Failure | Defense |
|---|---|
| Bridge silent death | `fix_priority0.bat` kills+restarts; doctor pings :6006 & :8654 |
| TZ Pro silent crash | `capture_daemon` polls every 5s, spawns/stops capture_v3 |
| `capture_v3` dies | Daemon respawns child each tick (`child.poll() is not None`) |
| Capture >15min stale | Tray tooltip shows `STALE: 18m`; `verify` subcommand returns nonzero |
| NMEA parse error | Bridge logs warning, keeps reading |
| Ship-log ingest fail | Non-blocking warning (logged but capture still succeeds) |
| Disk full | Capture fails atomically; PNG+JSON+MD either all exist or none |

## Module Dependency Map (Top-Level)

```
agent.py ────────┬── agent_loop.py ── agent_router.py ── providers/*
                 ├── analyzer.py ── providers/* ── vault.py
                 ├── vision.py ── providers/*
                 └── dashboard.py ── agent_router.py ── vessel.json

nmea_bridge.py ───── (no internal deps; ctypes + stdlib)
capture_daemon.py ──── capture_v3.py (subprocess only)
capture_v3.py ──────── nmea_bridge (reads :6006 for position)
tray_app.py ────────── capture_daemon (reads stampfile)
doctor.py ──────────── nmea_bridge, capture_daemon, dashboard, providers
```

## Privacy Boundary (Suit vs. Person)

The repository is the **suit** — what the world sees. Personal data, diary
entries, raw API keys, and the captain's voice notes live in
`~/tzpro-personal/`. The vault (`vault.py`) is the cryptographic gate
between suit and person. See `docs/SUIT_VS_PERSON.md`.

## Phase Mapping

| Phase | Doc | Status |
|---|---|---|
| Phase 1 — Capture / Tray / LAN Dashboard | `docs/phases/1.md` | **CURRENT** |
| Phase 1.5 — SPA pivot | `docs/phases/1.5.md` | planning |
| Phase 2 — Cascade loops | `docs/phases/2.md` | scaffolded |
| Phase 3 — Multi-modal sensing | `docs/phases/3.md` | designing |
| Phase 4 — Twin / projection | `docs/phases/4.md` | designing |
| Phase ∞ — Theia | `docs/phases/infinity.md` | vision |