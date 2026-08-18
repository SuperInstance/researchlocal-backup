"""tray_app.py — System-tray front door for the TZ Pro Agent.

When you double-click the desktop shortcut, this little app shows up
in the system tray with a small TZ Pro icon. Right-click reveals a
menu with the most common things the captain wants:

    Dashboard         (opens http://<lan_ip>:8090/ in the default browser)
    Folder: Captures  (opens the day's capture folder in Explorer)
    Folder: Logs      (opens logs/)
    Start capture     (runs `python capture_daemon.py run` if stopped)
    Stop capture      (runs `python capture_daemon.py stop` if running)
    Status            (polls daemon + bridge + dashboard + ollama)
    Refresh           (rebuild menu with current state)
    ─────────────
    Quit              (leaves the tray, stops nothing)

The tray itself is just a launcher — the heavy lifting is in:
  - capture_daemon.py (the lifecycle watcher that wraps capture_v3.py)
  - dashboard.py      (the LAN-reachable web UI)
  - doctor.py         (the system health check)

So we keep this script small and side-effect-light. It can crash and
the captain can just double-click the icon again.

Why pystray
-----------
pystray is the lightest cross-platform tray library for Python on
Windows. It uses the Windows Shell_NotifyIcon API directly via ctypes
(no extra compiled extension needed). Combined with PIL for icon work
and psutil for process queries, we have all we need without any heavy
UI framework like tkinter.
"""
from __future__ import annotations

import datetime as dt
import json
import logging
import os
import socket
import subprocess
import sys
import threading
import time
import webbrowser
from pathlib import Path
from typing import Optional

from PIL import Image
import psutil
from pystray import Icon, Menu, MenuItem

WORKSPACE = Path(__file__).resolve().parent
ICON_PATH = WORKSPACE / "assets" / "icon-tray-64.png"
DAEMON_SCRIPT = WORKSPACE / "capture_daemon.py"
DASHBOARD_PORT = 8090
LOG_DIR = WORKSPACE / "logs"
CAPTURES_DIR = WORKSPACE / "captures" / "v3"
STAMPFILE = WORKSPACE / "capture_daemon.stamp.json"
CAPTURE_INTERVAL_MIN = 10
# If the latest capture is older than interval + grace, the menu shows
# a STALE warning so the captain notices before losing a day's data.
STALE_THRESHOLD_S = (CAPTURE_INTERVAL_MIN + 5) * 60
LOCAL_TZ = dt.timezone(dt.timedelta(hours=-8))  # AKDT

LOG = logging.getLogger("tzpro_tray")


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

def _get_lan_ip() -> str:
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"


def _open_in_explorer(path: Path) -> None:
    """Open a folder in Windows Explorer, cross-platform fallback."""
    try:
        if sys.platform.startswith("win"):
            os.startfile(str(path))  # type: ignore[attr-defined]
        elif sys.platform == "darwin":
            subprocess.Popen(["open", str(path)])
        else:
            subprocess.Popen(["xdg-open", str(path)])
    except Exception as exc:
        LOG.error("could not open %s: %s", path, exc)


def _run_daemon_subcommand(args: list[str]) -> tuple[int, str]:
    """Run capture_daemon.py with the given subcommand. Returns (rc, output)."""
    try:
        cp = subprocess.run(
            [sys.executable, "-u", str(DAEMON_SCRIPT), *args],
            cwd=str(WORKSPACE),
            capture_output=True, text=True, timeout=10.0,
        )
        return cp.returncode, (cp.stdout or "") + (cp.stderr or "")
    except subprocess.TimeoutExpired:
        return -1, "timeout"
    except Exception as exc:
        return -2, str(exc)


# ---------------------------------------------------------------------------
# state probes
# ---------------------------------------------------------------------------

def _latest_capture_file() -> tuple[Path | None, float | None]:
    """Return (path, mtime) of newest .png under captures/v3, or (None, None)."""
    if not CAPTURES_DIR.exists():
        return None, None
    latest_mtime: float | None = None
    latest_path: Path | None = None
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
                    latest_path = png
    except OSError:
        return None, None
    return latest_path, latest_mtime


def _today_capture_dir() -> Path | None:
    """Return the captures dir for today (local AKDT), or None if missing."""
    today = dt.datetime.now(LOCAL_TZ).strftime("%Y-%m-%d")
    if not CAPTURES_DIR.exists():
        return None
    # capture_v3 names day folders like 2026-07-23_5547.492N_13141.083W
    for d in CAPTURES_DIR.iterdir():
        if d.is_dir() and d.name.startswith(today):
            return d
    return None


def _format_age(seconds: float | None) -> str:
    if seconds is None:
        return "never"
    if seconds < 60:
        return f"{seconds:.0f}s ago"
    if seconds < 3600:
        return f"{seconds/60:.0f}m ago"
    if seconds < 86400:
        return f"{seconds/3600:.1f}h ago"
    return f"{seconds/86400:.1f}d ago"


def daemon_state() -> dict:
    """Returns a small dict summarizing everything the menu needs."""
    s = {"daemon": "STOPPED", "capture": "STOPPED", "tzpro": "DOWN",
         "tzpro_pid": None, "lan_ip": _get_lan_ip(),
         "dashboard_url": f"http://{_get_lan_ip()}:{DASHBOARD_PORT}",
         "last_capture_at": None, "last_capture_age_s": None,
         "last_capture_path": None, "stale": False}
    if STAMPFILE.exists():
        try:
            data = json.loads(STAMPFILE.read_text(encoding="utf-8"))
        except Exception:
            data = {}
        if psutil.pid_exists(data.get("pid") or 0):
            s["daemon"] = "RUNNING"
        if psutil.pid_exists(data.get("child_pid") or 0):
            s["capture"] = "RUNNING"
    # Find the most recent capture on disk (cheaper than trusting stampfile
    # because the stampfile might be stale or missing).
    latest_path, latest_mtime = _latest_capture_file()
    if latest_mtime is not None:
        s["last_capture_at"] = dt.datetime.fromtimestamp(
            latest_mtime, tz=dt.timezone.utc).isoformat()
        s["last_capture_age_s"] = time.time() - latest_mtime
        s["last_capture_path"] = str(latest_path)
    # TZ Pro
    for proc in psutil.process_iter(["name", "pid"]):
        try:
            if (proc.info.get("name") or "").lower() == "timezero.exe":
                s["tzpro"] = "RUNNING"
                s["tzpro_pid"] = proc.info["pid"]
                break
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    # Stale = TZ Pro running but no capture in the threshold window.
    if s["tzpro"] == "RUNNING":
        age = s["last_capture_age_s"]
        if age is None or age > STALE_THRESHOLD_S:
            s["stale"] = True
    return s


def _status_line() -> str:
    st = daemon_state()
    age = st["last_capture_age_s"]
    stale_flag = "  STALE" if st["stale"] else ""
    return (
        f"TZ Pro: {st['tzpro']}  |  "
        f"Daemon: {st['daemon']}  |  "
        f"Capture: {st['capture']}  |  "
        f"Last: {_format_age(age)}{stale_flag}"
    )


# ---------------------------------------------------------------------------
# menu actions
# ---------------------------------------------------------------------------


def action_open_dashboard(icon: Icon, item: MenuItem) -> None:
    st = daemon_state()
    webbrowser.open(st["dashboard_url"])


def action_open_today_captures(icon: Icon, item: MenuItem) -> None:
    """Open today's captures folder (local AKDT date)."""
    today_dir = _today_capture_dir()
    if today_dir is None:
        LOG.warning("no captures dir for today yet")
        # Fall back to the captures root so the captain at least sees the tree.
        if CAPTURES_DIR.exists():
            _open_in_explorer(CAPTURES_DIR)
        return
    _open_in_explorer(today_dir)


def action_open_latest_capture(icon: Icon, item: MenuItem) -> None:
    """Open the most recent .png in the default image viewer."""
    latest_path, _ = _latest_capture_file()
    if latest_path is None:
        LOG.warning("no captures yet")
        return
    try:
        if sys.platform.startswith("win"):
            os.startfile(str(latest_path))  # type: ignore[attr-defined]
        elif sys.platform == "darwin":
            subprocess.Popen(["open", str(latest_path)])
        else:
            subprocess.Popen(["xdg-open", str(latest_path)])
    except Exception as exc:
        LOG.error("could not open %s: %s", latest_path, exc)


def action_open_logs(icon: Icon, item: MenuItem) -> None:
    _open_in_explorer(LOG_DIR if LOG_DIR.exists() else WORKSPACE)


def action_open_captures_root(icon: Icon, item: MenuItem) -> None:
    """Open the captures/v3 root for browsing the whole dataset."""
    if CAPTURES_DIR.exists():
        _open_in_explorer(CAPTURES_DIR)
    else:
        LOG.warning("captures dir does not exist: %s", CAPTURES_DIR)


def action_verify_captures(icon: Icon, item: MenuItem) -> None:
    """Run `python capture_daemon.py verify` in a new console so the
    captain can read the verdict (HEALTHY / STALE / UNHEALTHY)."""
    try:
        subprocess.Popen(
            [sys.executable, "-u", str(DAEMON_SCRIPT), "verify"],
            cwd=str(WORKSPACE),
            creationflags=getattr(subprocess, "CREATE_NEW_CONSOLE", 0),
        )
    except Exception as exc:
        LOG.error("could not open verify: %s", exc)


def action_start_daemon(icon: Icon, item: MenuItem) -> None:
    rc, out = _run_daemon_subcommand(["run", "--auto"])
    LOG.info("start daemon rc=%s out=%s", rc, out.strip())


def action_stop_daemon(icon: Icon, item: MenuItem) -> None:
    rc, out = _run_daemon_subcommand(["stop"])
    LOG.info("stop daemon rc=%s out=%s", rc, out.strip())


def action_status(icon: Icon, item: MenuItem) -> None:
    """Refresh the tray title to reflect current state."""
    st = daemon_state()
    age = st["last_capture_age_s"]
    stale_flag = "  [STALE]" if st["stale"] else ""
    icon.title = (
        f"TZ Pro Agent ({st['tzpro']}/{st['daemon']}/{st['capture']})\n"
        f"Last capture: {_format_age(age)}{stale_flag}\n"
        f"{st['dashboard_url']}"
    )
    icon.update_menu()
    LOG.info("status: %s", _status_line())


def action_refresh(icon: Icon, item: MenuItem) -> None:
    """Force a menu rebuild so the ✓ marks are current."""
    LOG.info("refresh: %s", _status_line())
    icon.menu = _build_menu(icon)
    icon.update_menu()


def action_quit(icon: Icon, item: MenuItem) -> None:
    LOG.info("quit requested from menu")
    icon.stop()


def action_open_doctor(icon: Icon, item: MenuItem) -> None:
    """Run `python doctor.py check` in a separate console so the
    captain can read the human-readable output."""
    try:
        subprocess.Popen(
            [sys.executable, "-u", "doctor.py", "check"],
            cwd=str(WORKSPACE),
            creationflags=getattr(subprocess, "CREATE_NEW_CONSOLE", 0),
        )
    except Exception as exc:
        LOG.error("could not open doctor: %s", exc)


# ---------------------------------------------------------------------------
# dynamic menu (checkmarks reflect state)
# ---------------------------------------------------------------------------

def _build_menu(icon: Icon) -> Menu:
    st = daemon_state()
    # These two items show checkmarks based on whether the daemon or
    # capture subsystem is running. Clicking Start when not running
    # launches it; clicking Stop when running stops it cleanly.
    daemon_running = st["daemon"] == "RUNNING"
    capture_running = st["capture"] == "RUNNING"
    age = st["last_capture_age_s"]
    stale_flag = "  STALE" if st["stale"] else ""

    last_capture_label = f"Last capture: {_format_age(age)}{stale_flag}"

    return Menu(
        MenuItem(f"Status: {st['tzpro']}/{st['daemon']}/{st['capture']}",
                 None, enabled=False),
        MenuItem(last_capture_label, None, enabled=False),
        MenuItem(f"Dashboard: {st['dashboard_url']}", action_open_dashboard,
                 default=True),
        Menu.SEPARATOR,
        MenuItem(
            "Start capture",
            action_start_daemon,
            visible=not daemon_running,
        ),
        MenuItem(
            "Stop capture",
            action_stop_daemon,
            visible=daemon_running,
        ),
        Menu.SEPARATOR,
        MenuItem("Open today's captures",   action_open_today_captures),
        MenuItem("Open latest capture",     action_open_latest_capture),
        MenuItem("Open all captures",       action_open_captures_root),
        MenuItem("Open logs folder",        action_open_logs),
        Menu.SEPARATOR,
        MenuItem("Verify capture health",   action_verify_captures),
        MenuItem("Doctor check",            action_open_doctor),
        MenuItem("Refresh menu",            action_refresh),
        Menu.SEPARATOR,
        MenuItem("Quit",                    action_quit),
    )


# ---------------------------------------------------------------------------
# periodic refresh worker
# ---------------------------------------------------------------------------

def _refresh_loop(icon: Icon) -> None:
    """Rebuild menu every 30s so checkmarks stay in sync.

    pystray's `update_menu()` redraws from whatever Menu object is
    currently assigned to `icon.menu`. To get a dynamic menu we build
    a fresh Menu here and assign it in place before triggering the
    redraw.
    """
    import time as _t
    while True:
        try:
            _t.sleep(30.0)
            fresh = _build_menu(icon)
            icon.menu = fresh
            icon.update_menu()
        except Exception:
            return


# ---------------------------------------------------------------------------
# entrypoint
# ---------------------------------------------------------------------------

def _autostart_daemon() -> None:
    """If the capture daemon isn't running, start it. This way the
    one-click industrial flow works: double-click the tray icon, and
    data starts flowing. The daemon itself only spawns capture_v3 when
    TZ Pro is actually up, so this is safe to do unconditionally."""
    st = daemon_state()
    if st["daemon"] == "RUNNING":
        LOG.info("daemon already running (pid lookup via stampfile)")
        return
    LOG.info("autostart: launching capture_daemon.py run --auto")
    rc, out = _run_daemon_subcommand(["run", "--auto"])
    LOG.info("autostart rc=%s out=%s", rc, out.strip())


def main() -> int:
    logging.basicConfig(level=logging.INFO,
                        format="%(asctime)s [%(levelname)s] %(message)s",
                        stream=sys.stdout)
    if not ICON_PATH.exists():
        print(f"icon asset missing at {ICON_PATH}", file=sys.stderr)
        return 1
    image = Image.open(ICON_PATH)
    st = daemon_state()

    # Industrial-grade: opening the tray icon = data starts flowing.
    # Run on a background thread so the tray icon shows up immediately.
    t_autostart = threading.Thread(target=_autostart_daemon,
                                   daemon=True, name="tzpro-tray-autostart")
    t_autostart.start()

    age = st["last_capture_age_s"]
    stale_flag = "  [STALE]" if st["stale"] else ""

    icon = Icon(
        "tzpro-agent",
        icon=image,
        title=(
            f"TZ Pro Agent ({st['tzpro']}/{st['daemon']}/{st['capture']})\n"
            f"Last capture: {_format_age(age)}{stale_flag}\n"
            f"{st['dashboard_url']}"
        ),
        menu=_build_menu(None),
    )

    # Background state-refresh worker (every 30s).
    t = threading.Thread(target=_refresh_loop, args=(icon,),
                        daemon=True, name="tzpro-tray-refresh")
    t.start()

    print(f"TZ Pro Agent tray running ({st['dashboard_url']})")
    icon.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
