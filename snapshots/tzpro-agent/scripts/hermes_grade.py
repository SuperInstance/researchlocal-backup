"""scripts/hermes_grade.py - score Hermes's completed missions.

Hermes drops results in ~/hermes-nerve-center/completed/. This script
reads one, grades it against the rubric in HERMES_TRAINING.md, and
appends a row to her notebook (~/hermes-nerve-center/notebook.csv).

The notebook is her grade history. It's the only feedback signal
that compounds. Without it, every mission is her first.

Usage
-----
    # Grade a single completed mission
    python scripts/hermes_grade.py --mission hermes_tier2_001

    # Grade all completed missions in batch
    python scripts/hermes_grade.py --batch

    # Show current tier + rolling averages
    python scripts/hermes_grade.py --status

Tier grading rules
------------------
Tier 0 (classify): exact match on source, |importance - answer| <= 0.1
    -> score = 1.0 if both, 0.5 if one, 0.0 if neither
Tier 1 (describe): manual scoring required; this script just records
    -> if completed file has 'captain_score: 0|1|2|3', record it
Tier 2 (anomaly precision@3): auto if 'precision_at_3' in result
Tier 3 (vision): auto if 'agreement_with_captain: true/false'
Tier 4 (correlate): auto if 'correlation_type' + 'narrative_score'
Tier 5 (hypothesize): auto if 'useful: true/false'

If a mission doesn't expose a known auto-grade key, we record it as
'ungraded' and Casey scores by hand later.

Promotion rules
---------------
A tier is "passed" when the rolling average of the last 20 scores at
that tier is >= 0.7. Tier promotion is logged but never automatic -
Casey reads the notebook and decides when Hermes is ready.

Output
------
Writes one row per mission to ~/hermes-nerve-center/notebook.csv:
    timestamp,task_id,tier,score,notes

Plus prints a one-line summary so the calling agent can paste it
into a SESSION-NOTE.
"""
from __future__ import annotations

import argparse
import csv
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

NERVE_CENTER = Path(os.environ.get("HERMES_NERVE_CENTER",
                                   r"C:\Users\casey\hermes-nerve-center"))
COMPLETED = NERVE_CENTER / "completed"
NOTEBOOK = NERVE_CENTER / "notebook.csv"


def _utcnow_iso() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def _ensure_notebook() -> None:
    if not NOTEBOOK.exists():
        with NOTEBOOK.open("w", newline="", encoding="utf-8") as f:
            w = csv.writer(f)
            w.writerow(["timestamp", "task_id", "tier", "score", "notes"])


def _read_mission(task_id: str) -> tuple[dict, Path]:
    """Find the completed mission file by task_id prefix."""
    candidates = sorted(COMPLETED.glob(f"{task_id}*.json"))
    if not candidates:
        raise FileNotFoundError(f"no completed mission matching {task_id!r}")
    if len(candidates) > 1:
        # Take the most recent
        candidates = sorted(candidates, key=lambda p: p.stat().st_mtime, reverse=True)
    p = candidates[0]
    with p.open("r", encoding="utf-8") as f:
        return json.load(f), p


def _grade_tier0(result: dict) -> tuple[float, str]:
    """Exact match on source + importance within tolerance."""
    expected = result.get("expected_answer") or {}
    want_source = expected.get("source")
    want_importance = expected.get("importance")
    got_source = result.get("classified_source")
    got_importance = result.get("classified_importance")
    if got_source is None or got_importance is None:
        return 0.0, "tier0: missing classified_source or classified_importance"
    src_ok = got_source == want_source
    imp_ok = abs(float(got_importance) - float(want_importance)) <= 0.1
    if src_ok and imp_ok:
        return 1.0, f"tier0: source={got_source}, imp={got_importance}"
    if src_ok or imp_ok:
        return 0.5, f"tier0: partial (src_ok={src_ok}, imp_ok={imp_ok})"
    return 0.0, f"tier0: wrong (got {got_source}/{got_importance}, want {want_source}/{want_importance})"


def _grade_tier1(result: dict) -> tuple[float, str]:
    """Captain already scored in the result file."""
    score = result.get("captain_score")
    if score is None:
        return -1.0, "tier1: needs captain_score 0..3"
    return float(score) / 3.0, f"tier1: captain_score={score}/3"


def _grade_tier2(result: dict) -> tuple[float, str]:
    """Precision@3 on known anomaly set."""
    p = result.get("precision_at_3")
    r = result.get("recall_at_3")
    if p is None:
        return -1.0, "tier2: needs precision_at_3 in result"
    return float(p), f"tier2: precision@3={p}, recall@3={r}"


def _grade_tier3(result: dict) -> tuple[float, str]:
    """Agreement with captain's log entry."""
    agree = result.get("agreement_with_captain")
    if agree is None:
        return -1.0, "tier3: needs agreement_with_captain bool"
    return 1.0 if agree else 0.0, f"tier3: agreement={agree}"


def _grade_tier4(result: dict) -> tuple[float, str]:
    """Correlation type match + narrative 0..3."""
    type_match = result.get("correlation_type_match")
    narrative = result.get("narrative_score")
    if type_match is None or narrative is None:
        return -1.0, "tier4: needs correlation_type_match + narrative_score"
    # 0.5 weight on type match, 0.5 weight on narrative/3
    return 0.5 * (1.0 if type_match else 0.0) + 0.5 * (float(narrative) / 3.0), \
           f"tier4: type_match={type_match}, narrative={narrative}/3"


def _grade_tier5(result: dict) -> tuple[float, str]:
    """Captain marks hypothesis useful or not."""
    useful = result.get("useful")
    if useful is None:
        return -1.0, "tier5: needs useful bool"
    return 1.0 if useful else 0.0, f"tier5: useful={useful}"


GRADERS = {
    0: _grade_tier0,
    1: _grade_tier1,
    2: _grade_tier2,
    3: _grade_tier3,
    4: _grade_tier4,
    5: _grade_tier5,
}


def grade_mission(task_id: str, verbose: bool = True) -> dict:
    """Grade one completed mission. Returns a summary dict."""
    _ensure_notebook()
    result, path = _read_mission(task_id)
    tier = int(result.get("tier", -1))
    grader = GRADERS.get(tier)
    if grader is None:
        raise ValueError(f"unknown tier {tier} in {path}")

    score, notes = grader(result)

    with NOTEBOOK.open("a", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow([_utcnow_iso(), task_id, tier, f"{score:.3f}", notes])

    summary = {
        "task_id": task_id,
        "tier": tier,
        "score": score,
        "notes": notes,
        "source_file": str(path),
    }
    if verbose:
        print(json.dumps(summary, indent=2))
    return summary


def grade_batch(verbose: bool = False) -> list[dict]:
    """Grade every completed mission that doesn't already have a notebook row."""
    _ensure_notebook()
    seen = set()
    if NOTEBOOK.exists():
        with NOTEBOOK.open("r", encoding="utf-8") as f:
            r = csv.DictReader(f)
            for row in r:
                seen.add(row["task_id"])
    results = []
    for path in sorted(COMPLETED.glob("*.json")):
        task_id = path.stem
        if task_id in seen:
            continue
        try:
            results.append(grade_mission(task_id, verbose=verbose))
        except Exception as exc:
            print(f"[!] could not grade {task_id}: {exc}", file=sys.stderr)
    return results


def show_status() -> None:
    """Print Hermes's current per-tier rolling averages."""
    if not NOTEBOOK.exists():
        print("notebook.csv not yet created - no missions graded")
        return
    by_tier: dict[int, list[float]] = {}
    with NOTEBOOK.open("r", encoding="utf-8") as f:
        r = csv.DictReader(f)
        for row in r:
            t = int(row["tier"])
            s = float(row["score"])
            by_tier.setdefault(t, []).append(s)
    if not by_tier:
        print("notebook is empty")
        return
    print(f"{'tier':<6}{'n':<6}{'avg_last_20':<14}{'latest':<10}")
    print("-" * 36)
    for tier in sorted(by_tier):
        scores = by_tier[tier][-20:]
        avg = sum(scores) / len(scores) if scores else 0.0
        latest = by_tier[tier][-1]
        print(f"{tier:<6}{len(by_tier[tier]):<6}{avg:<14.3f}{latest:<10.3f}")


def main() -> int:
    ap = argparse.ArgumentParser(description="Grade Hermes's completed missions")
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--mission", help="grade one completed mission by task_id")
    g.add_argument("--batch", action="store_true", help="grade all ungraded completed missions")
    g.add_argument("--status", action="store_true", help="show per-tier rolling averages")
    ap.add_argument("--quiet", action="store_true", help="suppress per-mission summary print")
    args = ap.parse_args()

    if args.status:
        show_status()
        return 0
    if args.batch:
        results = grade_batch(verbose=not args.quiet)
        print(f"\ngraded {len(results)} missions")
        return 0
    # --mission
    grade_mission(args.mission, verbose=not args.quiet)
    return 0


if __name__ == "__main__":
    sys.exit(main())