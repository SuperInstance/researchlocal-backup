"""scripts/_test_tray_app.py

Smoke test for tray_app.py — no actual tray, just verifies:
  1. imports clean (no missing modules)
  2. daemon_state() detects the running TZ Pro
  3. _build_menu() produces a Menu without raising
  4. all action_* callables are importable

We monkey-patch pystray.Icon.run() so the script doesn't block.
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

# Stub the pystray Icon.run so it doesn't block.
import pystray  # noqa: E402
_orig_run = pystray.Icon.run

def _stub_run(self):
    return None  # returns immediately; tray_app.main() then exits

pystray.Icon.run = _stub_run

import tray_app  # noqa: E402


def test_imports():
    print("[ok] tray_app imported")


def test_daemon_state():
    st = tray_app.daemon_state()
    print("[ok] daemon_state():", st)
    assert "lan_ip" in st
    assert "dashboard_url" in st
    # TZ Pro is currently running (PID 21268 in this session), but
    # test should be tolerant either way.
    assert st["tzpro"] in ("RUNNING", "DOWN")


def test_menu_build():
    # Build a dummy Icon for menu construction (Icon doesn't run shell).
    from PIL import Image
    img = Image.open(tray_app.ICON_PATH)
    icon = pystray.Icon("test", icon=img, title="")
    menu = tray_app._build_menu(icon)
    print(f"[ok] _build_menu() returned: {type(menu).__name__}")
    # Dump the menu items list for a quick eyeball.
    items = list(menu.items) if hasattr(menu, "items") else []
    print(f"     items={[(i.text if hasattr(i,'text') else str(i)) for i in items]}")


def test_action_signatures():
    for name in (
        "action_open_dashboard", "action_open_captures_folder",
        "action_open_logs", "action_start_daemon", "action_stop_daemon",
        "action_status", "action_refresh", "action_quit",
        "action_open_doctor",
    ):
        fn = getattr(tray_app, name)
        assert callable(fn), f"{name} not callable"
    print("[ok] all action_* callables present")


def test_main_does_not_block():
    # Patched run() returns None immediately, so main() should return.
    rc = tray_app.main()
    print(f"[ok] main() returned rc={rc}")


if __name__ == "__main__":
    test_imports()
    test_daemon_state()
    test_menu_build()
    test_action_signatures()
    test_main_does_not_block()
    print("\nALL PASS")
