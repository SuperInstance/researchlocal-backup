"""cascade/decaminute_loop.py — M10, the scribe (docs/17).

Reads the canonical 10-minute frame plus the racehorses' notes (mostly
for lat/lon — the spatial track of the time sequence), and writes THE
searchable record: word-based + structured JSON, vectorizable, never GC'd.
Also steers: may update the gaze for M1.
"""
from __future__ import annotations

import json
import logging
import time
from pathlib import Path

from . import config, gaze, ollama_client as oll, twin_sink

log = logging.getLogger("cascade.m10")

PROMPT = """You are the ship's log scribe on an Alaskan troller, writing the
canonical 10-minute sounder record.

You see: the current TZ Pro echogram frame, plus the watchstander's
minute notes from the last interval (their lat/lon track shows WHERE the
time-sequence in the echogram happened).

Write the record a searching fisherman would want. Respond with ONLY JSON:
{
 "summary": "2-3 sentences, pilot-house language",
 "bottom_fm": <number or null>,
 "bottom_type": "<hard|soft|mixed|unknown>",
 "schools": [{"depth_fm": <n>, "size": "<small|medium|large>", "band": "<LF|HF|both>"}],
 "thermocline_fm": <number or null>,
 "haze": "<none|light|heavy>",
 "anomalies": ["..."],
 "search_terms": ["chum", "feed layer", "bait ball", ...],
 "suggest_gaze": "<one-line focus for the watchstander next interval, or empty>"
}"""


def _have_vision_model() -> bool:
    """True iff a usable vision model is loaded. vision_available() just
    pings ollama; this checks the model registry too."""
    return (oll.model_present(config.MODEL_M10)
            or oll.model_present(config.MODEL_M1_FALLBACK))


def _build_sidecar_record(frame: Path, sidecar: dict, notes: list[dict]) -> dict:
    """Honest degraded-mode record. No sounder claims — only navigation
    facts present in the sidecar. Downstream briefings (H1, D1) see
    analysis_mode="sidecar_only" and brief accordingly (position track,
    motion, no sounder commentary).

    When vision lands later, real records supersede these by capture_id.
    The novel M1 notes (if any) carry M1's claims; this record only
    carries what the sidecar literally said.
    """
    pos = sidecar.get("position") or {}
    # M1 notes used: honest count, but no novel claims from them — sidecar
    # path means we are NOT incorporating vision-derived captions.
    m1_count = len(notes[-12:]) if notes else 0
    summary = (
        f"Sidecar-only record at {pos.get('lat_dd', '?'):.4f}, "
        f"{pos.get('lon_dd', '?'):.4f}" if pos.get('lat_dd') is not None
        else "Sidecar-only record (no position fix)"
    )
    if pos.get('sog_kts') is not None:
        summary += f"; SOG {pos['sog_kts']:.2f} kt"
    if m1_count:
        summary += f"; {m1_count} M1 note(s) on file (vision analysis pending)"
    return {
        "summary": summary,
        "bottom_fm": None,
        "bottom_type": "unknown",
        "schools": [],
        "thermocline_fm": None,
        "haze": "unknown",
        "anomalies": [],
        "search_terms": [],
    }


def write_record(frame: Path, sidecar: dict, notes: list[dict]) -> dict | None:
    if not oll.vision_available():
        log.info("ollama unavailable — M10 idling")
        return None

    # Determine path: vision-derived (preferred) vs sidecar-only (degraded).
    have_vision = _have_vision_model()

    parsed: dict = {}
    raw: str | None = None
    model_used = config.MODEL_M10 if have_vision else None

    if have_vision:
        track = [
            f"{n.get('ts_utc', '?')}: ({n.get('lat')}, {n.get('lon')}) — {n.get('caption', '')}"
            for n in notes[-12:]
        ]
        prompt = PROMPT + "\n\nWATCHSTANDER NOTES (newest last):\n" + (
            "\n".join(track) if track else "(none — interval had no new frames)"
        )
        raw = oll.vision_prompt(frame, prompt, config.MODEL_M10, config.M10_MAX_TOKENS,
                                 config.MODEL_M1_FALLBACK)
        if raw is None:
            log.warning("vision inference returned None — falling back to sidecar-only")
            parsed = _build_sidecar_record(frame, sidecar, notes)
            model_used = config.MODEL_M1_FALLBACK if oll.model_present(config.MODEL_M1_FALLBACK) else None
        else:
            parsed = oll.extract_json(raw) or {}
    else:
        # No vision model present. Be honest: emit a navigation-only record
        # so H1/D1 have position-track data to brief over, but mark the
        # analysis_mode explicitly so downstream never treats this as a
        # sounder claim.
        log.info("no vision model present — emitting sidecar-only record for %s", frame.name)
        parsed = _build_sidecar_record(frame, sidecar, notes)

    pos = sidecar.get("position") or {}
    frame_id = twin_sink.add_frame(frame, sidecar)
    record = {
        "spec": "echogram_record/1",
        "capture_id": sidecar.get("capture_id", frame.stem),
        "frame_id": frame_id,
        "ts_utc": sidecar.get("ts_utc") or time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "lat": pos.get("lat_dd"),
        "lon": pos.get("lon_dd"),
        "sog_kts": pos.get("sog_kts"),
        "cog_deg": pos.get("cog_deg"),
        "summary": parsed.get("summary", (raw or "")[:400]) if have_vision else parsed["summary"],
        "bottom_fm": parsed.get("bottom_fm"),
        "bottom_type": parsed.get("bottom_type", "unknown"),
        "schools": parsed.get("schools", []),
        "thermocline_fm": parsed.get("thermocline_fm"),
        "haze": parsed.get("haze", "unknown"),
        "anomalies": parsed.get("anomalies", []),
        "search_terms": parsed.get("search_terms", []),
        "m1_notes_used": len(notes[-12:]) if have_vision else 0,
        "model": model_used,
        # Degradation is explicit and machine-readable (docs/06 provenance).
        "analysis_mode": "vision" if have_vision and raw is not None else "sidecar_only",
    }

    out = config.DIR_RECORDS / f"{record['capture_id']}_record.json"
    tmp = out.with_suffix(".tmp")
    tmp.write_text(json.dumps(record, indent=2))
    tmp.replace(out)
    log.info("record written: %s", out.name)
    twin_sink.add_record(record)

    # Steer the racehorses: scribe may refocus the blinders for next interval.
    # Sidecar-only records have no sounder claims to suggest a gaze from;
    # only vision-derived records may steer.
    if record["analysis_mode"] == "vision":
        suggestion = (parsed.get("suggest_gaze") or "").strip()
        if suggestion:
            gaze.set_gaze(suggestion, set_by="M10", ttl_s=config.M10_INTERVAL * 2)

    return record


def main() -> None:
    import argparse
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
    p = argparse.ArgumentParser()
    p.add_argument("frame", type=Path, help="canonical 10-minute frame")
    p.add_argument("--notes", type=Path, default=None, help="JSONL of M1 notes")
    a = p.parse_args()
    config.ensure_dirs()

    sidecar = {}
    try:
        sidecar = json.loads(a.frame.with_suffix(".json").read_text())
    except Exception:
        pass
    notes = []
    if a.notes and a.notes.exists():
        notes = [json.loads(l) for l in a.notes.read_text().splitlines() if l.strip()]

    print(json.dumps(write_record(a.frame, sidecar, notes), indent=2))


if __name__ == "__main__":
    main()
