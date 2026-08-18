# Capture Daemon — `capture_daemon.py`

**File:** `capture_daemon.py` (~480 lines)
**Priority:** P1 — without this, echogram capture is unreliable.
**Owner:** tzpro-agent.

## What It Does

`capture_daemon.py` is the **TZ Pro lifecycle supervisor** for `capture_v3.py`.
The capture worker itself is dumb — it just captures every 10 minutes on the
hour boundary. The daemon adds three pieces of intelligence:

1. **Lifecycle awareness** — only run capture while `TimeZero.exe` is up.
2. **Self-healing** — if `capture_v3.py` dies, restart it.
3. **Visibility** — writes a stampfile the tray can read, plus a
   `verify` subcommand that prints a verdict for the captain.

## State Machine

```
            ┌─────────────────────────────────────────────────┐
            │                                                 │
            ▼                                                 │
   ┌─────────────────┐    TZ Pro first seen    ┌──────────────┴─────┐
   │    STOPPED      │ ────────────────────────►│   WATCHING_TZPRO   │
   │ (no stampfile)  │                         │  (child running)   │
   └─────────────────┘                         └──────────┬─────────┘
            ▲                                              │
            │  TZ Pro gone > 30 s grace                    │ TZ Pro gone
            │                                              ▼
            │                                  ┌───────────────────────┐
            └──────────────────────────────────│  GRACE (keep child    │
                stop child, unlink stampfile   │   alive for 30 s)    │
                                               └───────────────────────┘
```

The grace period (`GRACE_AFTER_TZPRO_DEATH_S = 30`,
`capture_daemon.py:67`) absorbs short TZ Pro restarts without losing
capture continuity.

## Subcommands

| Command | Behavior |
|---|---|
| `python capture_daemon.py run` | Long-lived watcher. Starts/stops child based on TZ Pro. |
| `python capture_daemon.py run --auto` | Same; tray uses this. Stampfile `mode="auto"`. |
| `python capture_daemon.py once` | Single capture then exit. Wrapper around `capture_v3.py --oneshot`. |
| `python capture_daemon.py stop` | SIGTERM the daemon and any child it owns; unlink stampfile. |
| `python capture_daemon.py status` | Multi-line state report. |
| `python capture_daemon.py doctor` | One-line health summary. |
| `python capture_daemon.py verify` | Captain-facing verdict (HEALTHY/STALE/UNHEALTHY/DEGRADED). Exit code 0 if healthy. |

## Stampfile (`capture_daemon.stamp.json`)

Written to repo root every 5 seconds (`capture_daemon.py:317-318`).
Atomic write via `.tmp` → rename (`capture_daemon.py:106-108`).

```json
{
  "pid": 12345,
  "child_pid": 12350,
  "started_at": "2026-07-24T16:00:00+00:00",
  "tzpro_pid": 7192,
  "tzpro_last": "2026-07-24T16:42:01+00:00",
  "last_capture_at": "2026-07-24T16:40:00+00:00",
  "mode": "auto"
}
```

| Field | Reader | Meaning |
|---|---|---|
| `pid` | tray, doctor | The daemon's PID. `_pid_alive()` checks it. |
| `child_pid` | tray, doctor | `capture_v3.py` subprocess PID, or null. |
| `started_at` | humans | When this daemon instance began. |
| `tzpro_pid` | humans, doctor | Last-seen TZ Pro PID. |
| `tzpro_last` | humans | ISO timestamp of most recent TZ Pro sighting. |
| `last_capture_at` | tray, doctor | ISO timestamp of newest `.png` on disk (refreshed each tick). |
| `mode` | humans | `"run"` \| `"once"` \| `"auto"` |

The stampfile is `.gitignore`d — it's transient runtime state.

## Lifecycle Loop

```
while not stop_flag:
    running, tz_pid = tzpro_running()
    if running:
        tzpro_last_seen_at = now
        if child is None or child.poll() is not None:
            child = launch_capture()    # spawn / respawn
        state["child_pid"] = child.pid
    else:
        # TZ Pro gone — apply grace period
        keep_alive = (now - tzpro_last_seen_at) < GRACE_AFTER_TZPRO_DEATH_S
        if not keep_alive and child alive:
            terminate_child(child)
    state["last_capture_at"] = latest_capture_at()
    _stamp_save(state)
    sleep 1s × 5 (interruptible by signal)
```

Key behaviors:

- **Child respawn** (`capture_daemon.py:291`): `child.poll() is not None`
  detects child death; daemon spawns a fresh one.
- **Grace period** (`capture_daemon.py:303-310`): If TZ Pro vanishes but
  we saw it within the last 30 s, we keep the child alive. This absorbs
  brief TZ Pro restarts (e.g. the captain hitting "Reload charts").
- **Interruptible sleep** (`capture_daemon.py:320-323`): Sleeps 1 s at a
  time, checking `stop_flag` each iteration, so SIGTERM is honored within
  a second rather than after up to 5 s.

## TZ Pro Detection

`tzpro_running()` (`capture_daemon.py:124-133`):

```python
for proc in psutil.process_iter(["name", "pid"]):
    try:
        nm = proc.info.get("name") or ""
        if nm.lower() == "timezero.exe":
            return True, proc.info["pid"]
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        continue
return False, None
```

psutil handles access-denied cases gracefully; we skip processes we can't
read.

## Subprocess Launch

`launch_capture()` (`capture_daemon.py:199-222`):

- Uses `python -u` (unbuffered) for prompt log delivery.
- `creationflags=CREATE_NO_WINDOW` so no console pop-up.
- Sidecar log at `logs/capture_v3.daemon-child.log` (append, line-buffered).
- `PYTHONUNBUFFERED=1` env var as belt-and-suspenders.

Termination (`terminate_child()`, `capture_daemon.py:225-243`) sends
SIGTERM, waits up to 10 s, then SIGKILL.

## `verify` Subcommand — Captain-Facing

```
TZ Pro:           RUNNING (pid=7192)
Daemon:           RUNNING
Capture child:    RUNNING
Latest capture:   142s ago  (stampfile says 2026-07-24T16:40:00+00:00)
Stale threshold:  900s (interval 10m + 5m grace)
VERDICT: HEALTHY
```

Verdict matrix (`capture_daemon.py:439-456`):

| Condition | Verdict | Exit code |
|---|---|---|
| TZ Pro up + daemon alive + child alive + freshness OK | `HEALTHY` | 0 |
| TZ Pro up + daemon down | `UNHEALTHY` | 1 |
| TZ Pro up + daemon alive + child alive + freshness BAD | `STALE` | 1 |
| TZ Pro up + daemon alive + child down | `DEGRADED` | 1 |
| TZ Pro down | `IDLE` | 0 |
| Anything else | `UNKNOWN` | 0 |

Tray menu calls this and surfaces the verdict.

## Boot via Tray

The tray (`tray_app.py`) auto-starts the daemon if its stampfile is missing:

```python
# tray_app.py — _ensure_daemon_running()
state = _stamp_load()
if not state or not psutil.pid_exists(state.get("pid")):
    subprocess.Popen(
        [sys.executable, "-u", "capture_daemon.py", "run", "--auto"],
        cwd=str(WORKSPACE),
        creationflags=CREATE_NO_WINDOW,
    )
```

So **opening the tray icon = capture flows**. This is the intended operator UX.

## Verified Line Numbers (as of `b64c2ef`)

- Constants & paths: `capture_daemon.py:62-71`
- Stampfile I/O: `capture_daemon.py:91-117`
- TZ Pro detection: `capture_daemon.py:124-133`
- Freshness probes: `capture_daemon.py:143-192`
- Subprocess control: `capture_daemon.py:199-243`
- Main loop: `capture_daemon.py:250-333`
- `verify` verdict matrix: `capture_daemon.py:408-456`

## Related Docs

- `docs/engineer/03_capture_v3.md` — the worker this daemon supervises
- `docs/engineer/04_tray_app.md` — the front door that auto-starts this
- `docs/engineer/06_doctor.md` — health checks this daemon participates in