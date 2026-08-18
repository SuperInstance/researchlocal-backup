"""next_action.py -- the yoke-move.

# ref: docs/BOOTCAMP.md rule 8 (write handoff when context gets heavy)
# ref: docs/PLANS/current.md (the plan this defends)
# ref: delegation/lesson_plan.py (where the NEXT bullet lives)

The harness ships a deterministic tool that returns the canonical
NEXT ACTION for the current scope (daily / weekly / phase). The model
reaches for "what should I do next?" often -- instead of arguing with
the model in the prompt, just give it a tool whose output is exactly
the answer the previous session left on disk.

This is the "move the yoke, not the model" principle from Round 3:
the harness controls the controls; the model doesn't have to learn
to ask differently.

Usage:
    python delegation/next_action.py             # default: daily NEXT
    python delegation/next_action.py --scope week
    python delegation/next_action.py --scope phase-1
    python delegation/next_action.py --scope monologue
    python delegation/next_action.py --json      # machine-readable
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from datetime import date
from pathlib import Path
from typing import Optional


WORKSPACE = Path(os.environ.get("TZPRO_WORKSPACE", r"C:\Users\casey\tzpro-agent"))
PLANS_DIR = WORKSPACE / "docs" / "PLANS"


def _daily_plan_path() -> Path:
    return PLANS_DIR / "daily" / f"{date.today().isoformat()}.md"


def _weekly_plan_path() -> Path:
    iso = date.today().isocalendar()
    return PLANS_DIR / "weekly" / f"{iso.year}-W{iso.week:02d}.md"


# Phase-scope files. Each scope maps to a doc with an implicit next
# action. The daily plan is the authoritative pointer; the phase docs
# are referenced from there.
PHASE_POINTERS = {
    "phase-1": [
        "docs/PLANS/daily/<TODAY>.md",
        "docs/ROADMAP.md",                  # phase-1 row in the index
        "docs/PHASES/phase-1.md",           # detail doc if present
    ],
    "phase-2": [
        "docs/PLANS/daily/<TODAY>.md",
        "docs/ROADMAP.md",                  # phase-2 row
        "docs/PHASES/phase-2.md",
    ],
    "monologue": [
        "docs/PLANS/daily/<TODAY>.md",
        "docs/BOOTCAMP.md",                 # rule 13
        "docs/BATON_PASS_2026-07-23.md",    # monologue design section
    ],
    "cost-throttle": [
        "docs/PLANS/daily/<TODAY>.md",
        "delegation/budget.py",
        "docs/BOOTCAMP.md",                 # rule 10
    ],
}


def _extract_next_bullet(plan_path: Path) -> Optional[str]:
    """Read the plan, return the single bullet under ## NEXT (if any)."""
    if not plan_path.exists():
        return None
    text = plan_path.read_text(encoding="utf-8")
    # Find "## NEXT" header, take the FIRST bullet line after it until
    # the next "## " header or EOF.
    m = re.search(r"^## NEXT\b.*?$", text, flags=re.MULTILINE)
    if not m:
        return None
    after = text[m.end():]
    next_section = re.search(r"^## ", after, flags=re.MULTILINE)
    block = after[: next_section.start()] if next_section else after
    for line in block.splitlines():
        stripped = line.strip()
        if stripped.startswith("- "):
            return stripped[2:].strip()
    return None


def _resolve(p: str) -> str:
    if "<TODAY>" in p:
        p = p.replace("<TODAY>", date.today().isoformat())
    return p


def get_next(scope: str = "daily") -> dict:
    """Return a dict describing the next action for the requested scope.

    Schema (stable, machine-readable):
        {
          "scope": "daily" | "weekly" | "phase-1" | ...,
          "plan_file": "<absolute path to the plan that sourced the bullet>",
          "next_action": "<the bullet text, or None if missing>",
          "pointers": [<files relevant to this scope>],
          "missing": True/False  # True if NEXT is empty
        }
    """
    if scope == "daily":
        plan = _daily_plan_path()
        pointers = [str(plan.relative_to(WORKSPACE))]
    elif scope == "weekly":
        plan = _weekly_plan_path()
        pointers = [str(plan.relative_to(WORKSPACE))]
    elif scope in PHASE_POINTERS:
        plan = _daily_plan_path()
        pointers = [_resolve(p) for p in PHASE_POINTERS[scope]]
    else:
        raise ValueError(f"unknown scope: {scope!r}")

    bullet = _extract_next_bullet(plan)
    return {
        "scope": scope,
        "plan_file": str(plan),
        "next_action": bullet,
        "pointers": pointers,
        "missing": bullet is None,
    }


def render(d: dict) -> str:
    """Human-readable rendering for terminal output."""
    if d["missing"]:
        out = [
            f"# Next action -- scope: {d['scope']}",
            "",
            "**No NEXT bullet is set in the active plan.**",
            "",
            f"Plan file: `{d['plan_file']}`",
            "",
            "Suggested recovery:",
            "    python -m delegation.lesson_plan.next_action \\",
            "        'set the NEXT bullet to whatever the right next step is'",
        ]
    else:
        out = [
            f"# Next action -- scope: {d['scope']}",
            "",
            f"> {d['next_action']}",
            "",
            f"Source: `{d['plan_file']}`",
            "",
            "Pointers:",
        ]
        for p in d["pointers"]:
            marker = "OK" if (WORKSPACE / p).exists() else "MISSING"
            out.append(f"  - `{p}`  [{marker}]")
    return "\n".join(out)


def main(argv: Optional[list[str]] = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[1])
    ap.add_argument(
        "--scope",
        default="daily",
        choices=["daily", "weekly", "phase-1", "phase-2", "monologue",
                 "cost-throttle"],
        help="which NEXT to surface (default: daily)",
    )
    ap.add_argument(
        "--json",
        action="store_true",
        help="emit JSON instead of human-readable text",
    )
    args = ap.parse_args(argv)
    d = get_next(args.scope)
    if args.json:
        print(json.dumps(d, indent=2))
    else:
        print(render(d))
    return 0


if __name__ == "__main__":
    sys.exit(main())