# Tray App — `tray_app.py`

**File:** `tray_app.py` (~440 lines)
**Priority:** UX — this is the captain's only direct interaction with
the system.
**Owner:** tzpro-agent.

## What It Does

A `pystray` system-tray icon that gives the captain one-click access to:

- The LAN dashboard
- Today's capture folder / latest capture / all captures
- Start / stop the capture daemon
- Verify capture health (calls `capture_daemon.py verify`)
- Doctor check
- Quit

The tray auto-starts the capture daemon on launch — so opening the
icon = data starts flowing. This is the **intended operator UX**.

## Icon Asset

`assets/icon-tray-64.png` (loaded at `tray_app.py:53, 423`). 64×64
RGBA. If missing, the tray exits with code 1 (`tray_app.py:420-422`).

## The Menu (`_build_menu`, `tray_app.py:332-372`)

Built dynamically every 30 s by `_refresh_loop`. Each menu item may
show or hide based on system state:

```
Status: trolling/RUNNING/RUNNING
Last capture: 142s
Dashboard: http://192.168.1.42:8090/
─────────────
Stop capture                 (visible when daemon_running)
─────────────
Open today's captures
Open latest capture
Open all captures
Open logs folder
─────────────
Verify capture health
Doctor check
Refresh menu
─────────────
Quit
```

### Menu Item Reference

| Item | Handler | Action |
|---|---|---|
| `Status: …` | (disabled) | Live status header |
| `Last capture: …` | (disabled) | Freshness indicator |
| `Dashboard: …` | `action_open_dashboard` | Opens LAN dashboard in browser |
| `Start capture` | `action_start_daemon` | Spawns `capture_daemon.py run --auto` |
| `Stop capture` | `action_stop_daemon` | Sends SIGTERM to daemon |
| `Open today's captures` | `action_open_today_captures` | Explorer at today's day folder |
| `Open latest capture` | `action_open_latest_capture` | Opens newest `.png` in image viewer |
| `Open all captures` | `action_open_captures_root` | Explorer at `captures/v3/` |
| `Open logs folder` | `action_open_logs` | Explorer at `logs/` |
| `Verify capture health` | `action_verify_captures` | Runs `capture_daemon.py verify` in console |
| `Doctor check` | `action_open_doctor` | Runs `python doctor.py` in console |
| `Refresh menu` | `action_refresh` | Forces immediate rebuild |
| `Quit` | `action_quit` | Leaves tray (does not stop daemon) |

## `daemon_state()` — The State Snapshot

`tray_app.py:148` (approximate). Returns a dict used to drive menu
visibility and the status header:

```python
{
  "tzpro":       "RUNNING" | "STOPPED",
  "daemon":      "RUNNING" | "STOPPED",
  "capture":     "RUNNING" | "STOPPED",
  "dashboard_url": "http://192.168.1.42:8090/",
  "last_capture_at": "2026-07-24T16:40:00+00:00" | None,
  "last_capture_age_s": 142.0 | None,
  "last_capture_path": Path | None,
  "stale": bool,
}
```

- Reads `capture_daemon.stamp.json` directly.
- Probes TCP :8090 for dashboard reachability.
- Computes `stale = last_capture_age_s > STALE_THRESHOLD_S`
  (`STALE_THRESHOLD_S = (10 + 5) * 60 = 900 s`, `tray_app.py:62`).

## Freshness Probe

`_latest_capture_file()` (≈`tray_app.py:103`) walks `captures/v3/*/*.png`
to find the newest mtime. Same logic as the daemon, but kept in the
tray so it can show freshness without the daemon being up.

## LAN IP Detection

`_get_lan_ip()` (`tray_app.py:72-80`):

```python
s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
s.connect(("8.8.8.8", 80))   # any external IP; doesn't actually send
ip = s.getsockname()[0]
```

This is the standard "UDP-connect trick" — no packets are sent, but
the kernel fills in the local IP that would be used. The dashboard
URL is then `http://<lan_ip>:8090/`.

## Autostart Behavior

`_autostart_daemon()` (`tray_app.py:402-413`) runs on a background
thread at tray startup:

```python
st = daemon_state()
if st["daemon"] == "RUNNING":
    return
subprocess.Popen(["python", "capture_daemon.py", "run", "--auto"], ...)
```

This is the **single-click industrial flow**: double-click the icon →
daemon starts → if TZ Pro is up, capture begins within 5 s.

The daemon itself only spawns `capture_v3.py` once it sees
`TimeZero.exe`, so unconditionally autostarting is safe.

## Refresh Loop

`_refresh_loop()` (`tray_app.py:379-…`) rebuilds the menu every 30 s
in a daemon thread:

```python
while True:
    sleep(30)
    fresh = _build_menu(icon)
    icon.menu = fresh
    icon.update_menu()
```

This keeps checkmarks and freshness indicators fresh without the
captain having to click Refresh manually.

## Subprocess Helper

`_run_daemon_subcommand(args)` (≈`tray_app.py:88-100`) runs
`python capture_daemon.py <args> …` with `CREATE_NO_WINDOW` and
captures stdout. Used by `action_start_daemon`, `action_stop_daemon`,
`action_verify_captures`. Returns `(rc, output_text)`.

## Verified Line Numbers (as of `b64c2ef`)

- Constants: `tray_app.py:52-65`
- LAN IP helper: `tray_app.py:72-80`
- Freshness probes: ~`tray_app.py:103-145`
- `daemon_state`: ~`tray_app.py:148-200`
- `_build_menu`: `tray_app.py:332-372`
- `_refresh_loop`: `tray_app.py:379-…`
- `_autostart_daemon`: `tray_app.py:402-413`
- `main`: `tray_app.py:416-…`

## Verifying the Tray is Working

1. **Look for the icon** — bottom-right of the taskbar near the clock.
2. **Right-click → Status line** — should read
   `Status: <state>/RUNNING/<state>` (where state varies).
3. **Right-click → Verify capture health** — opens console; should
   print `VERDICT: HEALTHY` if TZ Pro is up and daemon is running.
4. **Right-click → Open latest capture** — opens the most recent
   `.png` in the default image viewer. If you see today's echogram,
   the pipeline is alive.
5. **Tray tooltip** — pystray doesn't support dynamic tooltips on
   Windows, so this is in the menu instead.

## Common Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| Tray icon doesn't appear | `assets/icon-tray-64.png` missing | Restore the asset |
| Menu shows `STALE` | Bridge / TZ Pro / capture child down | Verify via menu item |
| "Start capture" doesn't start | Already running (or stuck) | Use Stop → Start sequence |
| LAN IP shows 127.0.0.1 | No network on this machine | Expected for offline vessels; dashboard still works locally |
| Tray crashes immediately | pystray / Pillow / psutil mismatch | Reinstall in venv |

## Related Docs

- `docs/engineer/02_capture_daemon.md` — what the tray starts
- `docs/engineer/06_doctor.md` — the doctor check the tray invokes
- `docs/BOAT_RUNBOOK.md` — captain-facing operator instructions