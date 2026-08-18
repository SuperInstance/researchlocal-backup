"""capture_daemon.py — TZ-Pro-aware controller for capture_v3.

Why this exists
---------------
`capture_v3.py` is a great 10-minute boundary-aligned capture loop, but
it has no opinion about whether TZ Pro is actually running. If the
captain goes to bed and shuts the wheelhouse down, the existing
capture_v3 keeps printing "no NMEA" errors all night and writes
nothing useful.

This daemon watches for the TZ Pro process (TimeZero.exe) and only
runs capture_v3 while it's alive. It can also be wired to a tray app
that pauses/resumes manually.

Modes
-----
    python capture_daemon.py run            # forever: starts when TZ Pro
                                           #  is seen, stops when TZ Pro dies
    python capture_daemon.py run --auto    # same; used by tray_app.py
                                           #  to react to lifecycle events
    python capture_daemon.py once          # capture NOW and exit
    python capture_daemon.py stop          # gracefully terminate any
                                           #  capture launched by *this*
                                           #  daemon (matched by stampfile)
    python capture_daemon.py status        # report current state to stdout
    python capture_daemon.py doctor        # print a quick health line

State machine
-------------
    STOPPED ──(TZ Pro seen)──→ RUNNING ──(TZ Pro gone / SIGTERM)──→ STOPPED

The daemon also writes a small stampfile next to vessel_state.jsonl so
multiple instances can't fight over the same capture slot. The stampfile
records:
    {
      "pid":            <this daemon's pid>,
      "child_pid":      <capture_v3 subprocess pid, or null>,
      "started_at":     <ISO timestamp>,
      "tzpro_pid":      <TimeZero.exe pid when last seen>,
      "tzpro_last":     <ISO timestamp of last TZ Pro sighting>,
      "last_capture_at": <ISO timestamp of newest .png on disk, or null>,
      "mode":           "run" | "once" | "auto"
    }

PID lookup uses psutil so we don't double-import Windows-only APIs.
"""
from __future__ import annotations

import argparse
import json
import logging
import os
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import psutil

WORKSPACE = Path(__file__).resolve().parent
STAMPFILE = WORKSPACE / "capture_daemon.stamp.json"
TZ_PRO_EXE = "TimeZero.exe"
CAPTURE_SCRIPT = WORKSPACE / "capture_v3.py"
CHECK_INTERVAL_S = 5          # poll interval for TZ Pro process
GRACE_AFTER_TZPRO_DEATH_S = 30  # keep capture_v3 alive this long
                                # after TZ Pro disappears (in case of
                                # brief restart) before stopping it
DEFAULT_CAPTURE_INTERVAL_MIN = 10  # mirror of capture_v3.CAPTURE_INTERVAL_MIN
LOG = logging.getLogger("capture_daemon")


# ---------------------------------------------------------------------------
# logging
# ---------------------------------------------------------------------------

def _setup_logging() -> None:
    level = logging.INFO
    logging.basicConfig(
        level=level,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        stream=sys.stdout,
    )


# ---------------------------------------------------------------------------
# stampfile (small JSON state next to vessel_state.jsonl)
# ---------------------------------------------------------------------------

def _stamp_paths() -> tuple[Path, Path]:
    """Return (live, lock) stampfile paths so we don't double-launch."""
    return STAMPFILE, STAMPFILE.with_suffix(STAMPFILE.suffix + ".lock")


def _stamp_load() -> dict | None:
    if not STAMPFILE.exists():
        return None
    try:
        return json.loads(STAMPFILE.read_text(encoding="utf-8"))
    except Exception:
        return None


def _stamp_save(state: dict) -> None:
    tmp = STAMPFILE.with_suffix(".tmp")
    tmp.write_text(json.dumps(state, indent=2), encoding="utf-8")
    tmp.replace(STAMPFILE)


def _pid_alive(pid: int | None) -> bool:
    if not pid:
        return False
    try:
        return psutil.pid_exists(pid)
    except Exception:
        return False


# ---------------------------------------------------------------------------
# TZ Pro detection
# ---------------------------------------------------------------------------

def tzpro_running() -> tuple[bool, int | None]:
    """Return (is_running, pid). Looks up TimeZero.exe case-insensitively."""
    for proc in psutil.process_iter(["name", "pid"]):
        try:
            nm = proc.info.get("name") or ""
            if nm.lower() == TZ_PRO_EXE.lower():
                return True, proc.info["pid"]
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    return False, None


# ---------------------------------------------------------------------------
# capture freshness probes
# ---------------------------------------------------------------------------

CAPTURES_DIR = WORKSPACE / "captures" / "v3"


def latest_capture_at() -> str | None:
    """Return ISO timestamp of the newest .png under captures/v3, or None.

    Cheap: walks the directory tree once and tracks the max mtime. We
    cap the walk at ~2 levels deep (year/day/file) which matches the
    capture_v3 layout so we don't blow up on huge datasets.
    """
    if not CAPTURES_DIR.exists():
        return None
    latest_mtime: float | None = None
    try:
        # capture_v3 layout: captures/v3/<YYYY-MM-DD>_<lat>_<lon>/<HHMM>_*.png
        for day_dir in CAPTURES_DIR.iterdir():
            if not day_dir.is_dir():
                continue
            for png in day_dir.glob("*.png"):
                try:
                    mt = png.stat().st_mtime
                except OSError:
                    continue
                if latest_mtime is None or mt > latest_mtime:
                    latest_mtime = mt
    except OSError:
        return None
    if latest_mtime is None:
        return None
    return datetime.fromtimestamp(latest_mtime, tz=timezone.utc).isoformat()


def freshness_seconds() -> float | None:
    """Return age in seconds of the newest capture, or None if no captures."""
    if not CAPTURES_DIR.exists():
        return None
    latest_mtime: float | None = None
    try:
        for day_dir in CAPTURES_DIR.iterdir():
            if not day_dir.is_dir():
                continue
            for png in day_dir.glob("*.png"):
                try:
                    mt = png.stat().st_mtime
                except OSError:
                    continue
                if latest_mtime is None or mt > latest_mtime:
                    latest_mtime = mt
    except OSError:
        return None
    if latest_mtime is None:
        return None
    return max(0.0, time.time() - latest_mtime)


# ---------------------------------------------------------------------------
# subprocess control
# ---------------------------------------------------------------------------

def launch_capture() -> subprocess.Popen:
    """Spawn capture_v3 as a child process. Uses pythonw.exe so the
    child has no console window. Stdout/stderr go to a sidecar log so
    the daemon can correlate."""
    sidecar = WORKSPACE / "logs" / "capture_v3.daemon-child.log"
    sidecar.parent.mkdir(parents=True, exist_ok=True)
    # Open in append, line-buffered so we can tail in another shell.
    log_fh = open(sidecar, "ab", buffering=0)

    env = os.environ.copy()
    # Force unbuffered output from the child.
    env["PYTHONUNBUFFERED"] = "1"

    kwargs = dict(
        args=[sys.executable, "-u", str(CAPTURE_SCRIPT)],
        stdout=log_fh,
        stderr=log_fh,
        stdin=subprocess.DEVNULL,
        cwd=str(WORKSPACE),
        env=env,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    LOG.info("launching capture_v3 (sid=%s)", sidecar)
    return subprocess.Popen(**kwargs)


def terminate_child(proc: subprocess.Popen, timeout_s: float = 10.0) -> bool:
    """Politely ask the child to terminate; SIGKILL if it ignores us."""
    if proc.poll() is not None:
        return True
    try:
        proc.terminate()
        proc.wait(timeout=timeout_s)
        return True
    except subprocess.TimeoutExpired:
        LOG.warning("child ignored SIGTERM after %.1fs; killing", timeout_s)
        proc.kill()
        try:
            proc.wait(timeout=5.0)
        except Exception:
            pass
        return True
    except Exception as exc:
        LOG.exception("error terminating child: %s", exc)
        return False


# ---------------------------------------------------------------------------
# main loops
# ---------------------------------------------------------------------------

def cmd_run(mode_label: str = "run") -> int:
    """Long-lived watcher. Stamps the stampfile, polls TZ Pro, manages
    capture_v3 as a child subprocess."""
    _setup_logging()
    my_pid = os.getpid()
    state = {
        "pid": my_pid,
        "child_pid": None,
        "started_at": datetime.now(timezone.utc).isoformat(),
        "tzpro_pid": None,
        "tzpro_last": None,
        "last_capture_at": latest_capture_at(),
        "mode": mode_label,
    }
    _stamp_save(state)

    # signal handling — SIGTERM/SIGINT flips a flag we poll each tick
    stop_flag = {"v": False}

    def _handle_signal(signum, frame):
        LOG.info("caught signal %s; requesting shutdown", signum)
        stop_flag["v"] = True
    signal.signal(signal.SIGTERM, _handle_signal)
    signal.signal(signal.SIGINT, _handle_signal)

    child: subprocess.Popen | None = None
    tzpro_last_seen_at: float | None = None

    LOG.info("daemon up (pid=%d, mode=%s, stamp=%s)",
             my_pid, mode_label, STAMPFILE)

    while not stop_flag["v"]:
        running, tz_pid = tzpro_running()
        now = time.time()

        if running:
            tzpro_last_seen_at = now
            state["tzpro_pid"] = tz_pid
            state["tzpro_last"] = datetime.now(timezone.utc).isoformat()

            # Start a child if we don't already have one alive.
            if child is None or child.poll() is not None:
                try:
                    child = launch_capture()
                    state["child_pid"] = child.pid
                except OSError as exc:
                    LOG.error("couldn't launch capture_v3: %s", exc)
                    state["child_pid"] = None
            else:
                # Reconcile child pid if the wrapper was respawned.
                state["child_pid"] = child.pid
        else:
            # TZ Pro not seen. Apply grace period before killing child.
            keep_alive = (
                tzpro_last_seen_at is not None
                and (now - tzpro_last_seen_at) < GRACE_AFTER_TZPRO_DEATH_S
            )
            if not keep_alive and child is not None and child.poll() is None:
                LOG.info("TZ Pro gone and grace expired; stopping child (pid=%s)",
                         child.pid)
                terminate_child(child)
                state["child_pid"] = None
            elif tzpro_last_seen_at is None:
                LOG.debug("waiting for TZ Pro (TimeZero.exe)…")

        # Refresh last_capture_at cheaply each tick so the tray can
        # display how stale the most recent capture is.
        state["last_capture_at"] = latest_capture_at()
        _stamp_save(state)
        # Tick. Sleep in 1s slices so signals respond quickly.
        for _ in range(CHECK_INTERVAL_S):
            if stop_flag["v"]:
                break
            time.sleep(1.0)

    # Shutdown
    LOG.info("shutting down daemon")
    if child is not None and child.poll() is None:
        terminate_child(child)
    try:
        STAMPFILE.unlink(missing_ok=True)
    except Exception:
        pass
    return 0


def cmd_once() -> int:
    """Single capture then exit. Convenience; capture_v3 --oneshot does
    the actual work."""
    _setup_logging()
    rc = subprocess.call(
        [sys.executable, "-u", str(CAPTURE_SCRIPT), "--oneshot"],
        cwd=str(WORKSPACE),
    )
    return rc


def cmd_stop() -> int:
    """Find any daemon-stamped capture_v3 subprocess and stop it."""
    _setup_logging()
    s = _stamp_load()
    if not s:
        print("daemon state file not found; nothing to stop")
        return 0
    daemon_pid = s.get("pid")
    child_pid = s.get("child_pid")
    if _pid_alive(daemon_pid):
        try:
            os.kill(daemon_pid, signal.SIGTERM)
            print(f"signaled daemon pid={daemon_pid}")
        except Exception as exc:
            print(f"could not signal daemon: {exc}")
    if _pid_alive(child_pid):
        try:
            os.kill(child_pid, signal.SIGTERM)
            print(f"signaled capture_v3 child pid={child_pid}")
        except Exception as exc:
            print(f"could not signal child: {exc}")
    try:
        STAMPFILE.unlink(missing_ok=True)
    except Exception:
        pass
    return 0


def cmd_status() -> int:
    s = _stamp_load()
    if not s:
        print("daemon: STOPPED (no stampfile)")
    else:
        daemon_alive = _pid_alive(s.get("pid"))
        child_alive = _pid_alive(s.get("child_pid"))
        tzpro_alive, tz_pid = tzpro_running()
        print(f"daemon:   {'RUNNING' if daemon_alive else 'STOPPED'}  (pid={s.get('pid')})")
        print(f"capture:  {'RUNNING' if child_alive else 'STOPPED'}  (pid={s.get('child_pid')})")
        print(f"TZ Pro:   {'RUNNING' if tzpro_alive else 'STOPPED'}  (pid={tz_pid})")
        print(f"mode:     {s.get('mode')}")
        print(f"started:  {s.get('started_at')}")
        print(f"tzpro_last: {s.get('tzpro_last')}")
    return 0


def cmd_doctor() -> int:
    """Compact one-line health summary, same shape as other doctors."""
    s = _stamp_load()
    if not s:
        print("capture-daemon: STOPPED")
    else:
        daemon_alive = _pid_alive(s.get("pid"))
        child_alive = _pid_alive(s.get("child_pid"))
        tzpro_alive, tz_pid = tzpro_running()
        daemon = "OK" if daemon_alive else "STOPPED"
        child = "OK" if child_alive else "NO-CHILD"
        tzpro = "OK" if tzpro_alive else "DOWN"
        print(f"capture-daemon: daemon={daemon} child={child} tzpro={tzpro}({tz_pid})")
    return 0


def cmd_verify() -> int:
    """Tell the captain whether the capture pipeline is actually producing
    data. Returns non-zero if freshness is broken.

    Logic:
      - If TZ Pro is running and daemon is STOPPED → UNHEALTHY (the very
        bug we keep hitting).
      - If TZ Pro is running and the most recent capture is older than
        (interval + 5 min grace) → STALE.
      - Otherwise → HEALTHY (or report what's expected, e.g. "no TZ Pro").
    """
    s = _stamp_load()
    daemon_alive = bool(s) and _pid_alive(s.get("pid"))
    child_alive = bool(s) and _pid_alive(s.get("child_pid"))
    tzpro_alive, tz_pid = tzpro_running()

    age_s = freshness_seconds()
    stamp_age = s.get("last_capture_at") if s else None
    interval_grace_s = (DEFAULT_CAPTURE_INTERVAL_MIN + 5) * 60

    print(f"TZ Pro:           {'RUNNING' if tzpro_alive else 'DOWN'} (pid={tz_pid})")
    print(f"Daemon:           {'RUNNING' if daemon_alive else 'STOPPED'}")
    print(f"Capture child:    {'RUNNING' if child_alive else 'STOPPED'}")
    if age_s is None:
        print(f"Latest capture:   NONE on disk")
    else:
        print(f"Latest capture:   {age_s:.0f}s ago  "
              f"(stampfile says {stamp_age})")
    print(f"Stale threshold:  {interval_grace_s:.0f}s "
          f"(interval {DEFAULT_CAPTURE_INTERVAL_MIN}m + 5m grace)")

    healthy = True
    if tzpro_alive and not daemon_alive:
        print("VERDICT: UNHEALTHY - TZ Pro is up but the capture daemon is dead.")
        healthy = False
    elif tzpro_alive and daemon_alive and child_alive:
        if age_s is None or age_s > interval_grace_s:
            print("VERDICT: STALE - capture child alive but no recent files.")
            healthy = False
        else:
            print("VERDICT: HEALTHY")
    elif tzpro_alive and daemon_alive and not child_alive:
        print("VERDICT: DEGRADED - daemon alive but capture child not running.")
        healthy = False
    elif not tzpro_alive:
        print("VERDICT: IDLE - TZ Pro not running; nothing to capture.")
    else:
        print("VERDICT: UNKNOWN")
    return 0 if healthy else 1


# ---------------------------------------------------------------------------
# entrypoint
# ---------------------------------------------------------------------------

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[1])
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_run = sub.add_parser("run", help="watch TZ Pro and manage capture_v3")
    p_run.add_argument("--auto", action="store_true",
                       help="alias flag used by tray_app (no-op other than "
                            "changing the stampfile mode label)")
    p_once = sub.add_parser("once", help="single capture then exit")
    sub.add_parser("stop", help="stop the running daemon (if any)")
    sub.add_parser("status", help="print daemon state")
    sub.add_parser("doctor", help="one-line health summary")
    sub.add_parser("verify", help="check capture freshness; non-zero if stale")

    args = parser.parse_args(argv)

    if args.cmd == "run":
        return cmd_run(mode_label="auto" if args.auto else "run")
    if args.cmd == "once":
        return cmd_once()
    if args.cmd == "stop":
        return cmd_stop()
    if args.cmd == "status":
        return cmd_status()
    if args.cmd == "doctor":
        return cmd_doctor()
    if args.cmd == "verify":
        return cmd_verify()
    parser.error(f"unknown command {args.cmd}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
