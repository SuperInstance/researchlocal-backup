"""
doctor.py - Health checks and repairs for the tzpro-agent stack.

Subcommands
-----------
    python doctor.py check              # Report only. Exit 0 healthy, 1 degraded.
    python doctor.py fix                # Apply repairs for known issues.
    python doctor.py fix --yes          # Skip interactive confirmation.

Each check returns a CheckResult with:
    ok      : bool    - True if healthy
    name    : str     - short identifier
    detail  : str     - human-readable status
    fix     : Callable[[], tuple[bool, str]] | None  - repair action

When --fix is requested, every failing check that exposes a `fix` callable
is invoked. Each fix returns (success, message). The runner reports a
final tally and exits 1 if anything remained unrepaired.

This module is import-safe (no side effects on import). Run the CLI via:
    python doctor.py check
    python doctor.py fix
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import urllib.error
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Optional

# ---------------------------------------------------------------------------
# Paths and constants
# ---------------------------------------------------------------------------

PROJECT_ROOT = Path(__file__).resolve().parent
BRIDGE_SCRIPT = PROJECT_ROOT / "nmea_bridge.py"
HEARTBEAT_FILE = PROJECT_ROOT / ".last_nmea_heartbeat"
VESSEL_STATE = PROJECT_ROOT / "vessel_state.jsonl"
BRIDGE_LOG = PROJECT_ROOT / "bridge.out.log"
BRIDGE_ERR_LOG = PROJECT_ROOT / "bridge.err.log"

BRIDGE_TCP_PORT = 6006       # raw NMEA0183 - TZ Pro connects here
BRIDGE_HTTP_PORT = 8654      # /vessel, /ready, /stream, /health
BRIDGE_HTTP_BASE = f"http://127.0.0.1:{BRIDGE_HTTP_PORT}"
BRIDGE_SERIAL_PORT = "COM6"
BRIDGE_BAUD = 4800

HEARTBEAT_STALE_SECONDS = 30   # bridge considered dead if no heartbeat in 30s
HTTP_TIMEOUT_SECONDS = 3


# ---------------------------------------------------------------------------
# Result type
# ---------------------------------------------------------------------------

@dataclass
class CheckResult:
    name: str
    ok: bool
    detail: str
    fix: Optional[Callable[[], tuple[bool, str]]] = None

    def render(self, color: bool = True) -> str:
        mark = "OK  " if self.ok else "FAIL"
        if color:
            green = "\033[92m"
            red = "\033[91m"
            reset = "\033[0m"
            mark = f"{green}{mark}{reset}" if self.ok else f"{red}{mark}{reset}"
        return f"  [{mark}] {self.name:<28} {self.detail}"


# ---------------------------------------------------------------------------
# Helper utilities
# ---------------------------------------------------------------------------

def _http_get_json(path: str) -> Optional[dict]:
    """GET a JSON document from the bridge HTTP API. Returns None on failure."""
    url = f"{BRIDGE_HTTP_BASE}{path}"
    try:
        with urllib.request.urlopen(url, timeout=HTTP_TIMEOUT_SECONDS) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, ConnectionError, TimeoutError, json.JSONDecodeError, OSError):
        return None


def _process_listening_on(port: int) -> Optional[int]:
    """Return the PID listening on `port` (TCP4 LISTEN), or None."""
    try:
        out = subprocess.check_output(
            ["netstat", "-ano", "-p", "TCP"],
            text=True, errors="ignore", timeout=5,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, OSError):
        return None
    needle = f":{port}"
    for line in out.splitlines():
        # Example: "  TCP    0.0.0.0:6006    0.0.0.0:0    LISTENING    1234"
        if needle in line and "LISTENING" in line:
            parts = line.split()
            if len(parts) >= 5 and parts[-1].isdigit():
                return int(parts[-1])
    return None


def _pid_has_command_substring(pid: int, substring: str) -> bool:
    """True if the process with this PID has `substring` in its command line."""
    try:
        out = subprocess.check_output(
            ["wmic", "process", "where", f"ProcessId={pid}",
             "get", "CommandLine", "/value"],
            text=True, errors="ignore", timeout=5,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, OSError):
        return False
    return substring.lower() in out.lower()


def _heartbeat_age_seconds() -> Optional[float]:
    """Age of the heartbeat file in seconds, or None if missing/unreadable."""
    try:
        mtime = HEARTBEAT_FILE.stat().st_mtime
    except OSError:
        return None
    return time.time() - mtime


def _parse_heartbeat_iso() -> Optional[str]:
    """Return the heartbeat ISO timestamp string, if present."""
    try:
        return HEARTBEAT_FILE.read_text(encoding="utf-8").strip() or None
    except OSError:
        return None


def _start_bridge_windows() -> tuple[bool, str]:
    """Launch nmea_bridge.py as a background pythonw.exe process."""
    if not BRIDGE_SCRIPT.exists():
        return False, f"bridge script missing: {BRIDGE_SCRIPT}"

    # First, defensively kill anything still holding 6006/8654.
    for port in (BRIDGE_TCP_PORT, BRIDGE_HTTP_PORT):
        pid = _process_listening_on(port)
        if pid is not None:
            try:
                subprocess.run(
                    ["taskkill", "/f", "/pid", str(pid)],
                    check=False, capture_output=True, timeout=5,
                )
            except OSError:
                pass

    # Give the OS a moment to release the port.
    time.sleep(1.5)

    cmd = [
        "pythonw",
        str(BRIDGE_SCRIPT),
        "--port", BRIDGE_SERIAL_PORT,
        "--baud", str(BRIDGE_BAUD),
    ]
    try:
        # CREATE_NEW_PROCESS_GROUP = 0x00000200, DETACHED_PROCESS = 0x00000008
        flags = 0x00000008
        subprocess.Popen(
            cmd,
            cwd=str(PROJECT_ROOT),
            creationflags=flags,
            stdout=open(BRIDGE_LOG, "ab"),
            stderr=open(BRIDGE_ERR_LOG, "ab"),
            close_fds=True,
        )
    except OSError as exc:
        return False, f"failed to launch pythonw: {exc}"

    # Wait up to 10s for /ready to come back.
    for _ in range(20):
        time.sleep(0.5)
        payload = _http_get_json("/ready")
        if payload and payload.get("ready"):
            return True, "bridge online and reporting ready"

    return False, "bridge launched but /ready never returned true within 10s"


# ---------------------------------------------------------------------------
# Individual checks
# ---------------------------------------------------------------------------

def check_bridge_tcp_port() -> CheckResult:
    pid = _process_listening_on(BRIDGE_TCP_PORT)
    if pid is None:
        return CheckResult(
            name="bridge:tcp:6006",
            ok=False,
            detail=f"nothing listening on 127.0.0.1:{BRIDGE_TCP_PORT}",
            fix=lambda: _start_bridge_windows(),
        )
    return CheckResult(
        name="bridge:tcp:6006",
        ok=True,
        detail=f"PID {pid} listening on :{BRIDGE_TCP_PORT}",
    )


def check_bridge_http_api() -> CheckResult:
    health = _http_get_json("/health")
    if health is None:
        return CheckResult(
            name="bridge:http:/health",
            ok=False,
            detail=f"no response from {BRIDGE_HTTP_BASE}/health",
            fix=lambda: _start_bridge_windows(),
        )
    return CheckResult(
        name="bridge:http:/health",
        ok=True,
        detail=f"health endpoint reports {health.get('status', '?')}",
    )


def check_bridge_heartbeat_fresh() -> CheckResult:
    age = _heartbeat_age_seconds()
    if age is None:
        return CheckResult(
            name="bridge:heartbeat",
            ok=False,
            detail=f"no heartbeat file at {HEARTBEAT_FILE.name}",
            fix=lambda: _start_bridge_windows(),
        )
    if age > HEARTBEAT_STALE_SECONDS:
        iso = _parse_heartbeat_iso() or "?"
        return CheckResult(
            name="bridge:heartbeat",
            ok=False,
            detail=f"heartbeat {age:.1f}s old (>{HEARTBEAT_STALE_SECONDS}s); last={iso}",
            fix=lambda: _start_bridge_windows(),
        )
    iso = _parse_heartbeat_iso() or "?"
    return CheckResult(
        name="bridge:heartbeat",
        ok=True,
        detail=f"fresh ({age:.1f}s); last={iso}",
    )


def check_bridge_serial_open() -> CheckResult:
    """The /ready endpoint tells us whether COM6 is open and we have a fix."""
    payload = _http_get_json("/ready")
    if payload is None:
        return CheckResult(
            name="bridge:serial",
            ok=False,
            detail="cannot reach bridge; run bridge:tcp fix first",
        )
    if not payload.get("ready", False):
        return CheckResult(
            name="bridge:serial",
            ok=False,
            detail=f"bridge up but no fix: {payload}",
        )
    sats = payload.get("satellites", 0)
    fix_q = payload.get("fix_quality", 0)
    return CheckResult(
        name="bridge:serial",
        ok=True,
        detail=f"COM6 open, fix_q={fix_q}, sats={sats}",
    )


def check_vessel_state_recent() -> CheckResult:
    """vessel_state.jsonl should be growing.

    # ref: docs/BOOTCAMP.md rule 5 -- DOCKED state is valid.
    # ref: docs/BATON_PASS_2026-07-23.md -- the previous failure was a
    #       false positive because the boat was stationary.

    We read the last line to discover the vessel's motion regime. If
    the last classification is "docked" or "unknown", a stale stream is
    *expected* and we report ok=True with a note. If the last
    classification is "trolling"/"cruising"/"slow_cruise", a stale
    stream is a real alarm (the boat is moving but the bridge stopped
    emitting -- restart_bridge.bat is the fix).
    """
    if not VESSEL_STATE.exists():
        return CheckResult(
            name="state:jsonl",
            ok=False,
            detail=f"{VESSEL_STATE.name} missing",
        )
    try:
        mtime = VESSEL_STATE.stat().st_mtime
    except OSError as exc:
        return CheckResult(name="state:jsonl", ok=False, detail=str(exc))
    age = time.time() - mtime

    # Read the last line's state_class (best-effort; the file is JSONL).
    last_class = None
    try:
        with VESSEL_STATE.open("r", encoding="utf-8") as fh:
            # SEEK_END then a single readline -- cheap on a big file.
            fh.seek(0, os.SEEK_END)
            end = fh.tell()
            # Look back up to 16 KiB for the last newline. 16 KiB is
            # comfortably larger than any single JSONL row from
            # nmea_bridge.py (~300-400 bytes), so we'll always capture
            # the last full line even on multi-MB streams.
            look = min(end, 16384)
            fh.seek(end - look)
            tail = fh.read()
            # tail = "...last_full_line\n<partial_or_empty>"
            # rsplit once on "\n" gives [prefix, suffix]; the prefix
            # ends with the last FULL line, the suffix is whatever came
            # after the last newline (often empty if the file ends
            # with "\n", or a partial line if the writer crashed).
            prefix, _, _suffix = tail.rpartition("\n")
            last_line = prefix.rsplit("\n", 1)[-1] if prefix else tail
            if last_line.strip():
                rec = json.loads(last_line)
                last_class = (rec.get("state_class") or "").strip() or None
    except Exception:
        # If we can't parse, fall back to the old "any staleness = bad"
        # behavior so a malformed file still surfaces as a problem.
        last_class = None

    STATIONARY = {"docked", "unknown"}

    if age > 120 and (last_class is None or last_class not in STATIONARY):
        return CheckResult(
            name="state:jsonl",
            ok=False,
            detail=(f"last write {age:.0f}s ago; last class={last_class or '?'} "
                    f"(state stream appears stalled; restart_bridge.bat if "
                    f"boat is moving)"),
        )
    if age > 120:
        # Stale but acceptable: the boat is at the dock or unknown.
        return CheckResult(
            name="state:jsonl",
            ok=True,
            detail=(f"updated {age:.0f}s ago; last class={last_class} "
                    f"(stationary -- stale stream is expected)"),
        )
    return CheckResult(
        name="state:jsonl",
        ok=True,
        detail=f"updated {age:.1f}s ago; class={last_class or '?'}",
    )


def check_ollama_running() -> CheckResult:
    """The local model server should be up so the agent can think."""
    try:
        with urllib.request.urlopen("http://127.0.0.1:11434/api/tags",
                                     timeout=2) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except Exception as exc:
        return CheckResult(
            name="ollama:11434",
            ok=False,
            detail=f"ollama not reachable: {exc.__class__.__name__}",
        )
    models = [m.get("name", "?") for m in data.get("models", [])]
    return CheckResult(
        name="ollama:11434",
        ok=True,
        detail=f"{len(models)} models: {', '.join(models[:4])}{'...' if len(models) > 4 else ''}",
    )


def check_vault_roundtrip() -> CheckResult:
    """The encrypted credential vault should be readable and round-trippable.

    Tests by writing a sentinel secret, reading it back, and deleting it.
    If the vault doesn't exist yet (first run), it's reported ok=True with
    detail noting no secrets stored.
    """
    try:
        from vault import Vault, VaultNotFoundError  # type: ignore
    except Exception as exc:
        return CheckResult(
            name="vault:roundtrip",
            ok=False,
            detail=f"vault module unavailable: {exc.__class__.__name__}",
        )
    v = Vault()
    sentinel = "__doctor_sentinel__"
    try:
        names = v.list_names()
    except Exception as exc:
        return CheckResult(
            name="vault:roundtrip",
            ok=False,
            detail=f"vault unreadable: {exc.__class__.__name__}: {exc}",
        )
    try:
        v.set(sentinel, {"probe": True, "ts": os.environ.get("TZPRO_TEST", "0")})
        got = v.get(sentinel)
        if got.get("probe") is not True:
            return CheckResult(
                name="vault:roundtrip",
                ok=False,
                detail="vault round-trip mismatch (got != set)",
            )
        v.delete(sentinel)
    except Exception as exc:
        return CheckResult(
            name="vault:roundtrip",
            ok=False,
            detail=f"vault round-trip failed: {exc.__class__.__name__}: {exc}",
        )
    return CheckResult(
        name="vault:roundtrip",
        ok=True,
        detail=f"{len(names)} secret(s) stored; DPAPI + AES-GCM round-trip ok",
    )


# ---------------------------------------------------------------------------
# Phase 1 additions: dashboard, capture_daemon
# ---------------------------------------------------------------------------

DASHBOARD_URL = "http://127.0.0.1:8090/api/health"
DAEMON_STAMPFILE = PROJECT_ROOT / "capture_daemon.stamp.json"


def check_dashboard_alive() -> CheckResult:
    """The LAN dashboard should answer /api/health. If not, no captain
    or crew phone can see anything from this stack."""
    try:
        with urllib.request.urlopen(DASHBOARD_URL, timeout=3.0) as resp:
            if resp.status != 200:
                return CheckResult(
                    name="dashboard:8090",
                    ok=False,
                    detail=f"HTTP {resp.status} from {DASHBOARD_URL}",
                    fix=lambda: _start_dashboard_windows(),
                )
            payload = json.loads(resp.read().decode("utf-8"))
        vessel = payload.get("vessel", "?")
        providers = ",".join(payload.get("providers_enabled", [])) or "(none)"
        return CheckResult(
            name="dashboard:8090",
            ok=True,
            detail=f"vessel={vessel}; providers={providers}",
        )
    except Exception as exc:
        return CheckResult(
            name="dashboard:8090",
            ok=False,
            detail=f"unreachable: {type(exc).__name__}: {exc}",
            fix=lambda: _start_dashboard_windows(),
        )


def _start_dashboard_windows() -> tuple[bool, str]:
    """Launch dashboard.py as a hidden python.exe subprocess. The
    dashboard binds 0.0.0.0:8090 so phones on the LAN can hit it."""
    try:
        subprocess.Popen(
            [sys.executable, "-u", str(PROJECT_ROOT / "dashboard.py")],
            cwd=str(PROJECT_ROOT),
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL,
        )
        return True, "started dashboard.py (PID assigned)"
    except Exception as exc:
        return False, f"failed to start dashboard: {exc}"


def check_capture_daemon() -> CheckResult:
    """The TZ-Pro-aware daemon may be running or stopped. We just
    report current state — 'STOPPED' is not necessarily an error
    because the captain may be at the dock with TZ Pro off. So this
    check reports info, ok=True, and includes the daemon status in
    the detail string."""
    if not DAEMON_STAMPFILE.exists():
        return CheckResult(
            name="capture:daemon",
            ok=True,
            detail="STOPPED (no stampfile; use 'Start capture' from tray or "
                   "`python capture_daemon.py run --auto`)",
        )
    try:
        data = json.loads(DAEMON_STAMPFILE.read_text(encoding="utf-8"))
    except Exception:
        return CheckResult(
            name="capture:daemon",
            ok=False,
            detail="stampfile unreadable; consider `python capture_daemon.py stop`",
            fix=lambda: _stop_capture_daemon(),
        )
    daemon_alive = bool(data.get("pid")) and _pid_alive(data.get("pid"))
    child_alive  = bool(data.get("child_pid")) and _pid_alive(data.get("child_pid"))
    if not daemon_alive and not child_alive:
        return CheckResult(
            name="capture:daemon",
            ok=False,
            detail=f"stampfile present but processes dead (pid={data.get('pid')}, "
                   f"child_pid={data.get('child_pid')})",
            fix=lambda: _stop_capture_daemon(),
        )
    return CheckResult(
        name="capture:daemon",
        ok=True,
        detail=(
            f"daemon={'OK' if daemon_alive else 'STOPPED'} "
            f"capture={'OK' if child_alive else 'WAITING'} "
            f"mode={data.get('mode','?')} "
            f"tzpro_last={data.get('tzpro_last') or 'never'}"
        ),
    )


def _pid_alive(pid):
    """Lightweight pid_exists without importing psutil (doctor.py is
    imported by services on cold-boot where psutil may already be
    loaded — but we keep this helper local)."""
    if not pid:
        return False
    try:
        import psutil  # type: ignore
        return psutil.pid_exists(pid)
    except Exception:
        # Fallback: Windows-native OpenProcess
        if sys.platform.startswith("win"):
            import ctypes
            PROCESS_QUERY_LIMITED = 0x1000
            kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
            handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED, False, pid)
            if not handle:
                return False
            kernel32.CloseHandle(handle)
            return True
        return False


def _stop_capture_daemon() -> tuple[bool, str]:
    try:
        rc = subprocess.call(
            [sys.executable, "-u", str(PROJECT_ROOT / "capture_daemon.py"),
             "stop"],
            cwd=str(PROJECT_ROOT),
            timeout=10.0,
        )
        return rc == 0, f"capture_daemon.py stop -> {rc}"
    except Exception as exc:
        return False, f"capture_daemon stop failed: {exc}"


# ---------------------------------------------------------------------------
# Check registry and runner
# ---------------------------------------------------------------------------

ALL_CHECKS: list[Callable[[], CheckResult]] = [
    check_bridge_tcp_port,
    check_bridge_http_api,
    check_bridge_heartbeat_fresh,
    check_bridge_serial_open,
    check_vessel_state_recent,
    check_ollama_running,
    check_vault_roundtrip,
    check_dashboard_alive,
    check_capture_daemon,
]


def run_checks() -> list[CheckResult]:
    return [fn() for fn in ALL_CHECKS]


def render_summary(results: list[CheckResult]) -> str:
    ok = sum(1 for r in results if r.ok)
    failed = [r for r in results if not r.ok]
    lines = ["", "=" * 60]
    lines.append(f"  tzpro-agent doctor: {ok}/{len(results)} healthy")
    if failed:
        lines.append(f"  {len(failed)} failing:")
        for r in failed:
            lines.append(f"      - {r.name}: {r.detail}")
    lines.append("=" * 60)
    return "\n".join(lines)


def cmd_check(_args: argparse.Namespace) -> int:
    results = run_checks()
    for r in results:
        print(r.render())
    print(render_summary(results))
    return 0 if all(r.ok for r in results) else 1


def cmd_fix(args: argparse.Namespace) -> int:
    results = run_checks()
    for r in results:
        print(r.render())

    failing = [r for r in results if not r.ok]
    if not failing:
        print(render_summary(results))
        return 0

    repairable = [r for r in failing if r.fix is not None]
    unrepairable = [r for r in failing if r.fix is None]

    print()
    print(f"  {len(failing)} failing, {len(repairable)} auto-repairable, "
          f"{len(unrepairable)} need human attention")

    if not args.yes:
        try:
            ans = input("\n  Apply repairs? [y/N] ").strip().lower()
        except EOFError:
            ans = "n"
        if ans not in ("y", "yes"):
            print("  Aborted. No changes made.")
            return 1

    print()
    fixed: list[str] = []
    still_broken: list[str] = []

    for r in repairable:
        print(f"  -> repairing {r.name} ...", end=" ", flush=True)
        try:
            ok, msg = r.fix()
        except Exception as exc:  # pragma: no cover - defensive
            ok, msg = False, f"fix raised: {exc!r}"
        if ok:
            print(f"OK ({msg})")
            fixed.append(r.name)
        else:
            print(f"FAILED ({msg})")
            still_broken.append(f"{r.name}: {msg}")

    # Re-run the full suite so we report ground truth, not guesses.
    print()
    print("  Re-checking ...")
    results_after = run_checks()
    for r in results_after:
        print(r.render())

    print()
    print("=" * 60)
    print(f"  Repaired: {len(fixed)} ({', '.join(fixed) or '-'})")
    if still_broken:
        print(f"  Still failing: {len(still_broken)}")
        for s in still_broken:
            print(f"      - {s}")
    print("=" * 60)

    return 0 if all(r.ok for r in results_after) else 1


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="doctor",
        description="Health checks and repairs for tzpro-agent.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_check = sub.add_parser("check", help="run all checks, report only")
    p_check.set_defaults(func=cmd_check)

    p_fix = sub.add_parser("fix", help="run checks and apply repairs where possible")
    p_fix.add_argument("--yes", "-y", action="store_true",
                       help="apply repairs without interactive confirmation")
    p_fix.set_defaults(func=cmd_fix)

    return parser


def main(argv: Optional[list[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
