"""lesson_plan.py -- daily/weekly plan storage that auto-evolves.

# ref: docs/BOOTCAMP.md rule 8 + 9
# ref: docs/BATON_PASS_2026-07-23.md (the "lesson plan" analogy)

Plans live as markdown files in docs/PLANS/. Three scopes:

    docs/PLANS/daily/<YYYY-MM-DD>.md       -- today's working plan
    docs/PLANS/weekly/<YYYY-Www>.md        -- this week's trellis
    docs/PLANS/current.md                  -- pointer to active plan
                                            (the one a fresh agent
                                             reads first)

Plans are append-and-refine, never rewrite. Each plan has:
    - HEADER:    one-line goal, dates, owner
    - STATE:     what we know is true (from latest doctor run etc)
    - MINUTES:   what we said we'd do in this scope
    - ACTUAL:    what actually happened (filled in as we go)
    - DELTA:     where the plan needs to bend, learned from ACTUAL
    - NEXT:      concrete next-action pointers for whoever picks up

The DELTA section IS the lesson-plan adjustment the teacher makes at
end of day. NEXT is the "tomorrow: do X, then Y" pointer.

The point of all this: a fresh agent (or fresh me, after /clear)
reads docs/PLANS/current.md FIRST, follows the NEXT pointers, and
knows exactly what to do without scrolling terminal history.

# see: docs/PLANS/current.md (created on first run if missing)
"""
from __future__ import annotations

import datetime as dt
import os
from pathlib import Path
from typing import Optional


def plans_root() -> Path:
    root = Path(os.environ.get("TZPRO_WORKSPACE", r"C:\Users\casey\tzpro-agent"))
    p = root / "docs" / "PLANS"
    p.mkdir(parents=True, exist_ok=True)
    (p / "daily").mkdir(exist_ok=True)
    (p / "weekly").mkdir(exist_ok=True)
    return p


def _today() -> str:
    return dt.date.today().isoformat()


def _this_week() -> str:
    iso = dt.date.today().isocalendar()
    return f"{iso.year}-W{iso.week:02d}"


def _stamp() -> str:
    return dt.datetime.now().strftime("%Y-%m-%d %H:%M:%S")


# ---------------------------------------------------------------------------
# plan skeleton
# ---------------------------------------------------------------------------

PLAN_SKELETON = """\
# {title}

*Opened {stamp}. Owner: orchestrator (this agent).*

## STATE  (what we know is true)

<!-- Updated when doctor.py, git status, or other ground-truth runs. -->

## MINUTES  (what we said we'd do)

<!-- Written when the plan opens. Refined as we learn. -->

## ACTUAL  (what actually happened)

<!-- Filled in as the session progresses. Append-only. -->

## DELTA  (where the plan needs to bend)

<!-- The teacher's adjustment: learned from ACTUAL, informs NEXT. -->

## NEXT  (concrete next-action pointers)

<!-- First thing the next reader (human or agent) should do. -->
"""


def ensure_today() -> Path:
    """Open today's plan if it doesn't exist; return its path."""
    p = plans_root() / "daily" / f"{_today()}.md"
    if not p.exists():
        title = f"Daily plan -- {_today()}"
        p.write_text(PLAN_SKELETON.format(title=title, stamp=_stamp()),
                     encoding="utf-8")
    return p


def ensure_week() -> Path:
    p = plans_root() / "weekly" / f"{_this_week()}.md"
    if not p.exists():
        title = f"Weekly plan -- {_this_week()}"
        p.write_text(PLAN_SKELETON.format(title=title, stamp=_stamp()),
                     encoding="utf-8")
    return p


def ensure_pointer() -> Path:
    """Make sure docs/PLANS/current.md points at the active plans.

    # ref: docs/AGENT_OPERATING_MODEL.md (the doctrine; the captain's entry point)
    # ref: docs/PLANS/current.md (this file's persistent form)

    This function only updates the *pointer header* (timestamp + the
    today's/this-week's plan links). It preserves any prose body
    below the header. If the file does not exist, it is created
    with a minimal skeleton that delegates the doctrine to
    docs/AGENT_OPERATING_MODEL.md.

    Historical note: an earlier version of this function rewrote the
    entire file from a hardcoded template, which silently clobbered
    any prose Casey or the agent had added. That was a bug -- the
    pointer is metadata, not the body.
    """
    cur = plans_root() / "current.md"
    today_path = ensure_today()
    week_path = ensure_week()

    today_rel = f"daily/{today_path.name}"
    week_rel = f"weekly/{week_path.name}"
    today_link = f"[`{today_rel}`]({today_rel})"
    week_link = f"[`{week_rel}`]({week_rel})"

    if not cur.exists():
        body = (
            f"# Active plans -- updated {_stamp()}\n\n"
            f"- **Today's plan:** {today_link}\n"
            f"- **This week:** {week_link}\n\n"
            f"## Bootstrap (read in this order)\n\n"
            f"1. **Today's plan above** (read the NEXT section first -- "
            f"that's what previous-you was doing).\n"
            f"2. **This week above** (read the DELTA section -- that's "
            f"where the teacher adjusted the plan).\n"
            f"3. `docs/AGENT_OPERATING_MODEL.md` -- the doctrine. Who "
            f"you are, who you serve.\n"
            f"4. `docs/BOOTCAMP.md` -- the operating rules.\n"
            f"5. `docs/NAVIGATION.md` -- the cockpit map.\n"
            f"6. `docs/BATON_PASS_<latest>.md` -- most recent handoff.\n"
            f"7. `docs/ROADMAP.md` -- phase-1..9 + phase-infinity.\n\n"
            f"After reading, run `python doctor.py check`. Then pick "
            f"up the NEXT bullet from today's plan.\n"
        )
        cur.write_text(body, encoding="utf-8")
        return cur

    # File exists. Update only the pointer header (first few lines).
    # Body below the header (anything after the first blank line) is
    # preserved verbatim.
    text = cur.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)

    # Skip leading lines until we've seen the first blank line. That
    # blank line is the boundary between the header (which we replace)
    # and the body (which we preserve).
    body_start = 0
    for i, line in enumerate(lines):
        if line.strip() == "":
            body_start = i + 1
            break
    body = "".join(lines[body_start:])

    new_header = (
        f"# Active plans -- updated {_stamp()}\n"
        f"\n"
        f"- **Today's plan:** {today_link}\n"
        f"- **This week:** {week_link}\n"
    )
    cur.write_text(new_header + "\n" + body, encoding="utf-8")
    return cur


# ---------------------------------------------------------------------------
# append helpers
# ---------------------------------------------------------------------------

def note(text: str, *, scope: str = "daily") -> Path:
    """Append a SESSION-NOTE block to the active plan.

    Use this freely. Each note is timestamped and appended; nothing is
    rewritten. This is the safety mechanism when context gets heavy.
    """
    if scope == "daily":
        p = ensure_today()
    elif scope == "weekly":
        p = ensure_week()
    else:
        raise ValueError(f"scope must be daily|weekly, got {scope!r}")

    block = (
        f"\n### SESSION-NOTE -- {_stamp()}\n\n"
        f"{text.strip()}\n"
    )
    with p.open("a", encoding="utf-8") as f:
        f.write(block)
    ensure_pointer()
    return p


def next_action(text: str, *, scope: str = "daily") -> Path:
    """Append (or replace) the NEXT bullet for the active plan.

    NEXT is the most-actionable single pointer; it gets rewritten as
    we learn what the actual next action is. Use this when you finish
    a step and want the next reader to start at the right place.
    """
    if scope == "daily":
        p = ensure_today()
    elif scope == "weekly":
        p = ensure_week()
    else:
        raise ValueError(f"scope must be daily|weekly, got {scope!r}")

    body = p.read_text(encoding="utf-8")
    new_bullet = f"- {text.strip()}"

    # Find the "## NEXT" header line
    lines = body.splitlines(keepends=False)
    next_idx: Optional[int] = None
    for i, line in enumerate(lines):
        if line.startswith("## NEXT"):
            next_idx = i
            break

    if next_idx is None:
        # No NEXT section yet -- append one.
        if not body.endswith("\n"):
            body += "\n"
        body += f"\n## NEXT\n\n{new_bullet}\n"
    else:
        # Find the next "## " header (start of the FOLLOWING section),
        # or end of file. Everything between is the NEXT body content.
        end_idx = len(lines)
        for j in range(next_idx + 1, len(lines)):
            if lines[j].startswith("## "):
                end_idx = j
                break

        # Rebuild: keep everything up to and including the header line,
        # drop any existing bullet lines in the body, append new bullet.
        kept = lines[: next_idx + 1]  # includes the header line
        body_lines = lines[next_idx + 1 : end_idx]
        # Drop existing single bullet (the first "- " line if present)
        body_cleaned = []
        bullet_dropped = False
        for ln in body_lines:
            stripped = ln.strip()
            if not bullet_dropped and stripped.startswith("- "):
                bullet_dropped = True
                continue
            body_cleaned.append(ln)

        # Assemble: header + blank + new bullet + (rest of body after bullet) + remaining sections
        new_block = list(kept)
        new_block.append("")  # blank line after header
        new_block.append(new_bullet)
        # Preserve any non-bullet content from the original NEXT body
        for ln in body_cleaned:
            if ln.strip():
                new_block.append(ln)
        # Append the trailing sections (everything from end_idx onward)
        new_block.extend(lines[end_idx:])

        body = "\n".join(new_block) + "\n"

    p.write_text(body, encoding="utf-8")
    ensure_pointer()
    return p


def actual(text: str, *, scope: str = "daily") -> Path:
    """Append a SESSION-RESULT to ACTUAL. Append-only."""
    if scope == "daily":
        p = ensure_today()
    elif scope == "weekly":
        p = ensure_week()
    else:
        raise ValueError(f"scope must be daily|weekly, got {scope!r}")

    block = (
        f"\n#### RESULT -- {_stamp()}\n\n"
        f"{text.strip()}\n"
    )
    with p.open("a", encoding="utf-8") as f:
        f.write(block)
    ensure_pointer()
    return p


def delta(text: str, *, scope: str = "daily") -> Path:
    """Append a plan-bend note to DELTA."""
    if scope == "daily":
        p = ensure_today()
    elif scope == "weekly":
        p = ensure_week()
    else:
        raise ValueError(f"scope must be daily|weekly, got {scope!r}")

    block = (
        f"\n#### DELTA-NOTE -- {_stamp()}\n\n"
        f"{text.strip()}\n"
    )
    with p.open("a", encoding="utf-8") as f:
        f.write(block)
    ensure_pointer()
    return p


# ---------------------------------------------------------------------------
# bootstrap -- ensure today's + this week's + the pointer all exist
# ---------------------------------------------------------------------------

def bootstrap() -> tuple[Path, Path, Path]:
    """Ensure all three plan files exist; return (today, week, pointer)."""
    return ensure_today(), ensure_week(), ensure_pointer()


if __name__ == "__main__":
    today, week, cur = bootstrap()
    print(f"today:  {today}")
    print(f"week:   {week}")
    print(f"cur:    {cur}")