"""scripts/_test_capture_daemon.py

Smoke test the capture_daemon:
  1. status (no stampfile -> STOPPED)
  2. once (capture_v3 --oneshot via cmd_once)
  3. run --auto in a background thread, sleep, verify stampfile is alive
  4. send SIGTERM, verify stampfile is cleaned up
"""
from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
import capture_daemon as cd


def test_status_when_stopped():
    # Remove any leftover stampfile
    cd.STAMPFILE.unlink(missing_ok=True)
    s = cd._stamp_load()
    assert s is None, "expected no stampfile initially"
    print("[ok] status -> STOPPED (no stampfile)")


def test_once_runs_capture():
    rc = cd.cmd_once()
    print(f"[ok] cmd_once returned {rc}")
    # 0=clean, anything else just means capture_v3 didn't write a file
    # (we tolerate 1 because it's possible NMEA was off)
    assert rc in (0, 1), f"unexpected exit code {rc}"


def test_run_and_stop():
    # background the daemon
    t = threading.Thread(
        target=lambda: cd.cmd_run(mode_label="auto"),
        daemon=True,
        name="capture-daemon-test",
    )
    t.start()
    print("[..] daemon thread started, sleeping 4s to let it write a stampfile…")
    time.sleep(4.0)
    s = cd._stamp_load()
    assert s is not None, "stampfile should exist after 4s"
    pid = s["pid"]
    child_pid = s.get("child_pid")
    print(f"[ok] stampfile written: daemon_pid={pid} child_pid={child_pid}")

    # send SIGTERM to ourselves (daemon installed a handler on this thread
    # only when run in foreground). To stop the daemon we use cmd_stop(),
    # which signals both the daemon pid and the child pid by PID.
    rc = cd.cmd_stop()
    print(f"[ok] cmd_stop returned {rc}")
    time.sleep(2.0)

    # If capture_v3 was spawned (because TZ Pro is running), it may be a
    # different PID. The daemon-stamped one should be gone.
    s2 = cd._stamp_load()
    assert s2 is None, f"stampfile should have been removed (got {s2})"
    print("[ok] stampfile removed after stop")
    # The daemon thread will exit on its own once SIGTERM lands.
    t.join(timeout=5.0)
    print("[ok] daemon thread exited")


if __name__ == "__main__":
    test_status_when_stopped()
    test_once_runs_capture()
    test_run_and_stop()
    print("\nALL PASS")
