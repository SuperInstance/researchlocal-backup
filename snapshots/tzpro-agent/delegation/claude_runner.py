"""claude_runner.py -- orchestrator's bridge to local Claude Code.

# ref: docs/BOOTCAMP.md rule 4 (use the Claude Code harness)
# ref: docs/BATON_PASS_2026-07-23.md (delegation worked pre-collapse)

This module lets the orchestrator (this agent) spawn short-lived
Claude Code sub-sessions that operate non-interactively in --print
mode. We use Claude Code as a high-quality sub-brain for:

  - reviewing large diffs I just produced
  - expanding doc stubs into long-form prose
  - writing unit tests for files I just authored
  - cross-checking decisions against the README / docs/ tree

Wrapper contract
----------------
    from delegation.claude_runner import run, ask

    run("summarize this diff in 3 bullets ...")         -> str
    ask("is this catch except too broad?", json_schema)  -> dict

The wrapper is deliberately tiny -- it just shells out. We never
proxy file system or shell access; Claude Code has its own tools
already.

BUDGET AWARENESS (added 2026-07-23, post-collapse)
-------------------------------------------------
Per docs/BOOTCAMP.md rule 8: when the orchestrator's context gets
heavy, the harness refuses new big work and steers toward
delegation/commits/handoff. This module is where that steering
lives -- the model doesn't have to remember to delegate; the
harness insists.

The check happens BEFORE we spend the cost of spawning a Claude
Code subprocess. We refuse via TierRefused; the caller (the
orchestrator) decides whether to commit, write handoff, or both.
"""
from __future__ import annotations

import json
import os
import shlex
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional

from delegation.budget import get as budget, TierRefused


# ---------------------------------------------------------------------------
# config
# ---------------------------------------------------------------------------

CLAUDE_BIN = (
    shutil.which("claude")
    or r"C:\Users\casey\AppData\Roaming\Claude\claude-code\2.0.65\claude.exe"
)

# Default append-system-prompt for tzpro-agent subagent work.
# This anchors Claude Code to the same context the orchestrator has,
# so it doesn't have to re-derive the project from scratch each call.
TZPRO_SUBAGENT_SYSTEM_PROMPT = (
    "You are a subagent of the orchestrator for the F/V Eileen TZ Pro "
    "Agent project at C:\\Users\\casey\\tzpro-agent. The orchestrator "
    "already maintains vessel_state, providers/, agent_router.py, "
    "dashboard.py, capture_daemon.py, tray_app.py, doctor.py, and "
    "docs/ROADMAP.md with phase-1..9 + phase-infinity. Your job is "
    "to produce high-quality output for the specific task at hand. "
    "Do not edit code unless explicitly asked. Be concise; return "
    "structured output when given a JSON schema. Avoid hallucinating "
    "file paths you have not actually read. When reviewing code, "
    "use Read/Grep tools against the actual files.\n"
)


# ---------------------------------------------------------------------------
# core runner
# ---------------------------------------------------------------------------

def run(
    prompt: str,
    *,
    append_system: Optional[str] = None,
    extra_args: Optional[list[str]] = None,
    cwd: Optional[str | Path] = None,
    timeout_s: float = 240.0,
) -> str:
    """Run Claude Code non-interactively and return its final text.

    Args:
        prompt:           The user-visible prompt to send.
        append_system:    Extra system context layered on top of the
                          default TZPRO_SUBAGENT_SYSTEM_PROMPT. Use this
                          to remind Claude Code of context it can't see.
        extra_args:       Extra CLI flags to forward. Most useful:
                            --output-format json
                            --json-schema <file>
                            --allowed-tools "Read,Bash,Edit"
                            --permission-mode bypassPermissions
        cwd:              Working directory for the claude process.
        timeout_s:        Wall-clock ceiling before we kill it.

    Returns the raw final text. If --output-format=json was used,
    parse it via ask() instead; otherwise the JSON is returned as text
    and you must json.loads() it.
    """
    # Budget gate -- the yoke-move. At DARK/NIGHT we refuse new
    # delegation; the orchestrator must commit + write handoff first.
    b = budget()
    if not b.can("delegate"):
        raise TierRefused(b.why_not("delegate") or "delegate refused")

    sys_prompt = TZPRO_SUBAGENT_SYSTEM_PROMPT
    if append_system:
        sys_prompt += "\n" + append_system

    cmd = [
        CLAUDE_BIN,
        "-p",
        prompt,
        "--append-system-prompt", sys_prompt,
        "--output-format", "text",
        "--permission-mode", "bypassPermissions",
    ]
    if extra_args:
        cmd.extend(extra_args)

    try:
        cp = subprocess.run(
            cmd,
            cwd=str(cwd) if cwd else None,
            capture_output=True,
            text=True,
            timeout=timeout_s,
            env={**os.environ, "CLAUDE_CODE_DISABLE_TELEMETRY": "1"},
        )
    except subprocess.TimeoutExpired as exc:
        raise RuntimeError(f"claude timed out after {timeout_s}s") from exc

    if cp.returncode != 0 and not cp.stdout:
        raise RuntimeError(
            f"claude exit={cp.returncode}\n"
            f"stderr={cp.stderr[-800:] if cp.stderr else '(none)'}"
        )
    return cp.stdout or ""


def ask(
    prompt: str,
    json_schema: Optional[dict] = None,
    *,
    append_system: Optional[str] = None,
    cwd: Optional[str | Path] = None,
    timeout_s: float = 240.0,
) -> dict | list | str:
    """Like run() but returns parsed JSON when given a schema.

    The schema must be a valid JSON Schema object. Claude Code writes
    it to a temp file and passes --json-schema <path>. If no schema
    is given, the call is just like run() with --output-format=json.
    """
    extra_args = ["--output-format", "json"]
    if json_schema is not None:
        schema_path = Path(os.environ.get("TEMP", "/tmp")) / "_claude_schema.json"
        schema_path.write_text(json.dumps(json_schema), encoding="utf-8")
        extra_args.extend(["--json-schema", str(schema_path)])

    out = run(
        prompt,
        append_system=append_system,
        extra_args=extra_args,
        cwd=cwd,
        timeout_s=timeout_s,
    )
    try:
        return json.loads(out)
    except json.JSONDecodeError:
        # Some modes return JSON-wrapped result blocks. Best effort.
        return out


# ---------------------------------------------------------------------------
# health check -- runs in <5s, useful for "is Claude Code available"
# ---------------------------------------------------------------------------

def ping() -> tuple[bool, str]:
    """Cheap health check -- always bypasses the budget gate.

    The ping IS the gate's health check; if ping refused at the gate,
    we'd never know whether Claude Code was up. Bypass directly via
    subprocess; no `run()` recursion.
    """
    import subprocess
    try:
        cp = subprocess.run(
            [CLAUDE_BIN, "-p", "Reply with the single word: pong",
             "--output-format", "text",
             "--permission-mode", "bypassPermissions"],
            capture_output=True, text=True, timeout=30.0,
        )
        out = cp.stdout or ""
        return ("pong" in out.lower()), out.strip()
    except Exception as exc:
        return False, f"{type(exc).__name__}: {exc}"


# ---------------------------------------------------------------------------
# __all__
# ---------------------------------------------------------------------------

__all__ = ["run", "ask", "ping", "CLAUDE_BIN", "TZPRO_SUBAGENT_SYSTEM_PROMPT"]


if __name__ == "__main__":
    ok, msg = ping()
    if ok:
        print(f"claude subagent: OK ({msg!r})")
        sys.exit(0)
    print(f"claude subagent: FAILED ({msg})")
    sys.exit(1)