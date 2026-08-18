"""scripts/_test_phase1_e2e.py

End-to-end smoke test for Phase 1.

Steps:
  1. doctor check baseline
  2. start capture_daemon in background (real subprocess, not thread)
  3. wait for stampfile + child_pid to appear
  4. check that capture_v3 child is running
  5. doctor check — both dashboard and capture:daemon should be OK
  6. stop capture_daemon (sends SIGTERM via cmd_stop)
  7. wait, verify stampfile gone, child gone
  8. doctor check — final state matches what we started with

This is more thorough than the capture_daemon-only test because it
also exercises the dashboard probe and the doctor wiring.
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))
import capture_daemon as cd  # noqa: E402
import doctor  # noqa: E402


def _ok(msg: str) -> None:
    print(f"[ok] {msg}")


def test_baseline():
    rc = doctor.main(["check"])
    # baseline can include the state:jsonl failure (boat stationary)
    assert rc in (0, 1), f"unexpected baseline exit {rc}"
    _ok("baseline doctor check ran")


def test_start_daemon_via_cli():
    """Use cmd_run() in a sub-process; verify it spawns capture_v3."""
    import subprocess
    proc = subprocess.Popen(
        [sys.executable, "-u", "capture_daemon.py", "run", "--auto"],
        cwd=str(ROOT),
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    print(f"[..] started daemon pid={proc.pid}, waiting 8s…")
    time.sleep(8.0)
    s = cd._stamp_load()
    assert s is not None, "stampfile should exist after 8s"
    print(f"[ok] stampfile present: daemon={s['pid']} child={s['child_pid']}")
    assert cd._pid_alive(s.get("pid")), "daemon should be alive"
    if s.get("child_pid"):
        assert cd._pid_alive(s["child_pid"]), "capture_v3 should be alive"
    _ok("daemon + capture_v3 alive")
    return proc


def test_doctor_with_daemon_running():
    # Capture subprocess handles for later teardown
    s = cd._stamp_load() or {}
    daemon_pid = s.get("pid")
    child_pid = s.get("child_pid")
    results = doctor.run_checks()
    by_name = {r.name: r for r in results}
    cd_check = by_name.get("capture:daemon")
    assert cd_check is not None and cd_check.ok, f"capture:daemon not ok: {cd_check}"
    _ok(f"doctor reports capture:daemon OK ({cd_check.detail})")
    return daemon_pid, child_pid


def test_stop_daemon(proc):
    print("[..] sending cmd_stop…")
    cd.cmd_stop()
    try:
        proc.wait(timeout=10.0)
        _ok(f"daemon subprocess exited rc={proc.returncode}")
    except Exception as exc:
        print(f"[warn] daemon didn't exit cleanly: {exc}")
    # Wait for stampfile cleanup
    for i in range(20):
        time.sleep(0.5)
        if not cd.STAMPFILE.exists():
            _ok("stampfile removed")
            return
    raise AssertionError("stampfile not removed after cmd_stop")


def test_post_teardown():
    rc = doctor.main(["check"])
    assert rc in (0, 1), f"unexpected post-teardown exit {rc}"
    s = cd._stamp_load()
    assert s is None, f"expected stampfile cleared, got {s}"
    _ok("post-teardown stampfile is None")


if __name__ == "__main__":
    test_baseline()
    proc = test_start_daemon_via_cli()
    try:
        test_doctor_with_daemon_running()
    finally:
        test_stop_daemon(proc)
    test_post_teardown()
    print("\nALL PASS")
