"""scripts/hermes_send.py - drop a mission into Hermes's inbox.

Wraps the JSON packet format from README_API.md so we don't have to
hand-write it each time. The packet points Hermes at paths to read
(rather than embedding raw data) so inbox files stay small.

Usage
-----
    # Tier 0 mission: classify a Moment's source + importance
    python scripts/hermes_send.py --tier 0 --moment path/to/moment.json

    # Tier 1 mission: describe a Moment's payload
    python scripts/hermes_send.py --tier 1 --moment path/to/moment.json

    # Tier 2 mission: anomaly detection on a window of moments
    python scripts/hermes_send.py --tier 2 --moment-window path/to/window.jsonl \\
            --engine-window path/to/engine.jsonl --answer path/to/answer.json

The 'mission seed' script (`scripts/hermes_seed.py`) generates
synthetic Moments + known-answer sets so we can run tier-0 and
tier-2 missions before we have a real fishing season.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path

NERVE_CENTER = Path(os.environ.get("HERMES_NERVE_CENTER",
                                   r"C:\Users\casey\hermes-nerve-center"))
INBOX = NERVE_CENTER / "inbox"

INSTRUCTIONS = {
    0: ("Classify this Moment's source and importance. "
        "Read the JSON, look at payload keys + structure, "
        "respond with classified_source and classified_importance."),
    1: ("Describe this Moment in 2-3 sentences. "
        "Stick to facts in the payload. Don't invent."),
    2: ("Given the NMEA+engine windows, pick the top-3 most anomalous "
        "Moments. Read both files, return anomaly_ids ranked + "
        "precision_at_3 + recall_at_3 vs the answer file."),
    3: ("Look at the sounder image at <path>. Estimate bottom depth, "
        "mark density (none|sparse|moderate|dense), features, and "
        "confidence per claim. Return as structured JSON."),
    4: ("Given the participating Anomalies, classify the correlation_type "
        "and write a 1-sentence narrative. Return correlation_type "
        "and narrative."),
    5: ("Given the last 7 days of anomalies, propose one hypothesis. "
        "Cite supporting moment_ids. Return hypothesis + moment_ids + "
        "confidence. If you don't see a pattern, say so."),
}


def _utcnow_iso() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def _new_task_id(tier: int) -> str:
    return f"hermes_tier{tier}_{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S')}_{uuid.uuid4().hex[:6]}"


def _packet(tier: int, context: dict) -> dict:
    return {
        "task_id": _new_task_id(tier),
        "priority": "NORMAL",
        "category": "ANALYZE",
        "instruction": INSTRUCTIONS[tier],
        "context": {"tier": tier, **context},
        "metadata": {
            "origin": "mini-agent",
            "timestamp": _utcnow_iso(),
            "training_mode": True,
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="Drop a mission into Hermes's inbox")
    ap.add_argument("--tier", type=int, required=True, choices=[0, 1, 2, 3, 4, 5])
    ap.add_argument("--moment", help="path to a single Moment JSON file")
    ap.add_argument("--moment-window", help="path to a JSONL of Moments")
    ap.add_argument("--engine-window", help="path to a JSONL of engine Moments")
    ap.add_argument("--answer", help="path to the known-answer JSON for grading")
    ap.add_argument("--model", default="deepseek-ai/DeepSeek-V3-Flash",
                    help="DeepInfra model id to use")
    ap.add_argument("--dry-run", action="store_true",
                    help="print packet instead of writing to inbox/")
    args = ap.parse_args()

    if args.tier == 0:
        if not args.moment:
            ap.error("--moment required for tier 0")
        context = {"moment_path": str(Path(args.moment).resolve()),
                   "model_hint": args.model}
    elif args.tier == 1:
        if not args.moment:
            ap.error("--moment required for tier 1")
        context = {"moment_path": str(Path(args.moment).resolve()),
                   "model_hint": args.model}
    elif args.tier == 2:
        if not (args.moment_window and args.engine_window and args.answer):
            ap.error("--moment-window, --engine-window, and --answer required for tier 2")
        context = {
            "nmea_path": str(Path(args.moment_window).resolve()),
            "engine_path": str(Path(args.engine_window).resolve()),
            "answer_path": str(Path(args.answer).resolve()),
            "model_hint": args.model,
        }
    else:
        # Tier 3, 4, 5 use the moment path or window path generically
        context = {"moment_path": str(Path(args.moment).resolve()) if args.moment else None,
                   "model_hint": args.model}

    packet = _packet(args.tier, context)

    if args.dry_run:
        print(json.dumps(packet, indent=2))
        return 0

    INBOX.mkdir(parents=True, exist_ok=True)
    out = INBOX / f"{packet['task_id']}.json"
    with out.open("w", encoding="utf-8") as f:
        json.dump(packet, f, indent=2)
    print(f"dropped: {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())