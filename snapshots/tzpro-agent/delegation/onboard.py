"""onboard.py -- zero-shot navigator for fresh agents.

# ref: docs/BOOTCAMP.md (zero-shot navigability rule)
# ref: docs/BATON_PASS_2026-07-23.md (reconstruction test)

The hard test of a self-navigating codebase: a fresh agent with NO
prior context can pick the right next action by reading only files
that exist in the repo.

    python delegation/onboard.py                # default: pick scope
    python delegation/onboard.py phase-2        # focus on phase-2 work
    python delegation/onboard.py monologue      # focus on the monologue build
    python delegation/onboard.py cost-throttle  # focus on cost throttle
    python delegation/onboard.py debugging      # something's broken; help

The output is a literal "read this, then this, then this" sequence.
No inference required by the caller; the harness did the inference
once and committed it to docs/NAVIGATION.md.

This file is the yoke-move: when a fresh agent reaches for the
wrong starting file, this script is what they run, not a cleverer
prompt.

# see: docs/NAVIGATION.md (the actual file list per scope)
"""
from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Optional


WORKSPACE = Path(
    os.environ.get("TZPRO_WORKSPACE", r"C:\Users\casey\tzpro-agent")
)
NAV_FILE = WORKSPACE / "docs" / "NAVIGATION.md"

# ---------------------------------------------------------------------------
# scopes -> ordered file lists
# ---------------------------------------------------------------------------
#
# Each scope is a (label, ordered-list-of-files-to-read, summary).
# Files are relative to WORKSPACE. The order is the read order.
#
# The lesson-plan NAVIGATION.md is the persistent form of this table;
# edit the file, not the dict, if you want the change to stick past
# process exit. onboard.py reads NAVIGATION.md at startup if present.

DEFAULT_SCOPES: dict[str, dict] = {
    "default": {
        "summary": "Resume the boat-agent project at the latest checkpoint.",
        "files": [
            "docs/PLANS/current.md",
            "docs/PLANS/daily/<TODAY>.md",   # resolved at runtime
            "docs/PLANS/weekly/<THISWEEK>.md",
            "docs/BOOTCAMP.md",
            "docs/BATON_PASS_2026-07-23.md",
            "docs/ROADMAP.md",
        ],
    },
    "phase-1": {
        "summary": "Verify or extend Phase 1 (bridge, dashboard, tray, doctor).",
        "files": [
            "docs/PLANS/current.md",
            "doctor.py",
            "dashboard.py",
            "tray_app.py",
            "capture_daemon.py",
            "delegation/claude_runner.py",
            "docs/HARDWARE_SETUP.md",
        ],
    },
    "monologue": {
        "summary": "Build the tzpro-monologue internal-monologue agent.",
        "files": [
            "docs/PLANS/current.md",
            "docs/BOOTCAMP.md",
            "docs/BATON_PASS_2026-07-23.md",
            # The monologue design was pure talk; the persisted references
            # are in BOOTCAMP rule 13 and the BATON_PASS section.
        ],
    },
    "cost-throttle": {
        "summary": "Build the time-aware free-tier cost throttle.",
        "files": [
            "docs/PLANS/current.md",
            "delegation/budget.py",
            "docs/BOOTCAMP.md",   # rule 10
        ],
    },
    "debugging": {
        "summary": "Something is broken; find the right doctor check.",
        "files": [
            "docs/PLANS/current.md",
            "doctor.py",
            "docs/TROUBLESHOOTING.md",
        ],
    },
}


def _resolve(p: str) -> str:
    """Resolve placeholder tokens like <TODAY>."""
    if "<TODAY>" in p:
        from datetime import date
        p = p.replace("<TODAY>", date.today().isoformat())
    if "<THISWEEK>" in p:
        from datetime import date
        iso = date.today().isocalendar()
        p = p.replace("<THISWEEK>", f"{iso.year}-W{iso.week:02d}")
    return p


def resolve_scope(name: str) -> dict:
    """Look up a scope by name. Falls back to 'default'.

    Tries NAVIGATION.md first (custom scopes), then DEFAULT_SCOPES.
    """
    # NAVIGATION.md override -- if present, parse scopes from it
    if NAV_FILE.exists():
        # Lightweight parser: lines "## scope: <name>" followed by a
        # "summary:" line and a code-block of file paths.
        text = NAV_FILE.read_text(encoding="utf-8")
        scopes = _parse_navigation(text)
        if name in scopes:
            return scopes[name]
        if name == "default" and "default" in scopes:
            return scopes["default"]

    if name in DEFAULT_SCOPES:
        return DEFAULT_SCOPES[name]
    return DEFAULT_SCOPES["default"]


def _parse_navigation(text: str) -> dict[str, dict]:
    """Parse docs/NAVIGATION.md into a {name: scope_dict} mapping.

    Format expected (lightweight):
        ## scope: <name>
        summary: <one-line summary>
        ```
        relative/path/one.md
        relative/path/two.md
        ```
    """
    scopes: dict[str, dict] = {}
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if line.startswith("## scope:"):
            name = line.split(":", 1)[1].strip()
            summary = ""
            files: list[str] = []
            j = i + 1
            # collect until end of code block
            while j < len(lines):
                l = lines[j].rstrip()
                if l.startswith("summary:"):
                    summary = l.split(":", 1)[1].strip()
                elif l.strip() == "```":
                    j += 1
                    while j < len(lines) and lines[j].strip() != "```":
                        s = lines[j].strip()
                        if s and not s.startswith("#"):
                            files.append(s)
                        j += 1
                    break
                j += 1
            scopes[name] = {"summary": summary, "files": files}
            i = j + 1
        else:
            i += 1
    return scopes


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def onboard(scope: str = "default") -> str:
    """Return the read-this-next output for the requested scope."""
    s = resolve_scope(scope)
    out_lines = [
        f"# Onboard -- scope: {scope}",
        "",
        s["summary"],
        "",
        "## Read these files, in order",
        "",
    ]
    for i, f in enumerate(s["files"], 1):
        rel = _resolve(f)
        full = WORKSPACE / rel
        marker = "OK" if full.exists() else "MISSING"
        out_lines.append(f"{i}. `{rel}`  [{marker}]")
    out_lines.append("")
    out_lines.append("After reading, run:")
    out_lines.append("    python delegation/doctor.py   (or `python doctor.py`)")
    out_lines.append("to confirm the system is healthy, then commit before")
    out_lines.append("starting new work.")
    return "\n".join(out_lines)


if __name__ == "__main__":
    arg = sys.argv[1] if len(sys.argv) > 1 else "default"
    print(onboard(arg))