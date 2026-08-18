"""scripts/hermes_seed.py - generate synthetic Moments + answer keys.

We can't train Hermes against real fishing data until the boat is
out. To start her training today, generate synthetic Moments with
known answers:

  - moments.jsonl   - a sequence of NMEA + engine Moments
  - answer.json     - the right anomaly picks + tier-0 classification
                      for each Moment

Run modes:
  python scripts/hermes_seed.py tier0     # one Moment, easy to grade
  python scripts/hermes_seed.py tier2     # 60 min of NMEA+engine

Synthetic data rules
--------------------
Tier 0:
  - Source: round-robin through sounder, nmea, engine, thermal, audio,
    voice (so Hermes has to learn the schema differences, not memorize)
  - Importance: derived from payload richness (more keys = higher)

Tier 2:
  - 60 minutes of NMEA: lat/lon drifting slowly, then one 30s window
    where SOG spikes from 4.2 to 9.5kt (trawl coming out of water)
  - 60 minutes of engine: oil pressure steady ~60psi, then one 45s
    window where it dips to 42psi (alarm)
  - Answer: the two anomaly moment_ids + their timestamps
"""
from __future__ import annotations

import argparse
import json
import math
import random
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from schema import Moment, Position  # noqa: E402


def _utc(t: datetime) -> str:
    return t.astimezone(timezone.utc).isoformat(timespec="seconds")


def _ts(base: datetime, seconds: int) -> str:
    return _utc(base + timedelta(seconds=seconds))


def seed_tier0() -> dict:
    """One Moment, known-answer for classify + importance."""
    sources = ["sounder", "nmea", "engine", "thermal", "audio", "voice"]
    payloads = {
        "sounder": {"path": "/x.png", "width": 1920, "height": 1080, "hash": "abc"},
        "nmea":    {"sentence": "$GPRMC,123456,A,5547.123,N,13141.456,W,4.2,270,230726",
                    "parsed": {"sog": 4.2, "cog": 270}},
        "engine":  {"pgn": 127489, "fields": {"oil_pressure_psi": 60.0, "oil_temp_c": 85.0,
                                               "rpm": 1450, "hours": 1234.5}},
        "thermal": {"path": "/t.png", "min_c": 12.0, "max_c": 45.0, "mean_c": 28.0,
                    "ambient_c": 18.0},
        "audio":   {"rms": 0.12, "peak": 0.45, "centroid_hz": 380.0},
        "voice":   {"text": "Got one, port side, decent size", "stt_model": "whisper-large",
                    "duration_s": 4.2},
    }
    importance_by_source = {
        "sounder": 0.6, "nmea": 0.3, "engine": 0.7,
        "thermal": 0.8, "audio": 0.5, "voice": 0.9,
    }
    src = random.choice(sources)
    m = Moment(
        source=src,
        position=Position(lat=55.7833, lon=-132.4167, sog_knots=4.2, cog_deg=270),
        payload=payloads[src],
        importance=importance_by_source[src],
    )
    return {
        "moment": m.to_dict(),
        "answer": {
            "source": src,
            "importance": importance_by_source[src],
        },
    }


def seed_tier2() -> dict:
    """60 minutes of NMEA + engine with two planted anomalies."""
    base = datetime.now(timezone.utc).replace(microsecond=0, second=0, minute=0)
    nmea = []
    engine = []
    planted_nmea_ids = []
    planted_engine_ids = []

    # lat/lon drifting slowly (trawler moving NW at ~4.2kt)
    start_lat, start_lon = 55.7833, -132.4167
    for s in range(0, 3600, 30):  # every 30s for 60 min
        drift_nm = (4.2 / 3600) * s  # 4.2 kt = 4.2 nm/hour
        lat = start_lat + (drift_nm / 60.0) * 0.01
        lon = start_lon - (drift_nm / 60.0) * 0.01
        sog = 4.2
        if 1500 <= s <= 1530:  # 30s anomaly: trawl coming out
            sog = 9.5
            planted_nmea_ids.append(f"m_nmea_{s:05d}")
        nmea.append(Moment(
            id=f"m_nmea_{s:05d}",
            timestamp=_ts(base, s),
            source="nmea",
            position=Position(lat=round(lat, 5), lon=round(lon, 5),
                              sog_knots=sog, cog_deg=315, fix_quality="gps"),
            payload={"sentence": f"$GPRMC,..,A,{lat:.3f},N,{abs(lon):.3f},W,{sog},{315}",
                     "parsed": {"sog": sog, "cog": 315}},
            importance=0.3,
        ).to_dict())

        oil_p = 60.0
        oil_t = 85.0
        rpm = 1450
        if 1800 <= s <= 1845:  # 45s anomaly: oil pressure dip
            oil_p = 42.0
            planted_engine_ids.append(f"m_engine_{s:05d}")
        engine.append(Moment(
            id=f"m_engine_{s:05d}",
            timestamp=_ts(base, s),
            source="engine",
            payload={"pgn": 127489, "fields": {"oil_pressure_psi": oil_p,
                                                "oil_temp_c": oil_t,
                                                "rpm": rpm}},
            importance=0.4,
        ).to_dict())

    # The expected answer: the planted moments, in order
    answer_ids = planted_nmea_ids + planted_engine_ids
    return {
        "nmea": nmea,
        "engine": engine,
        "answer": {
            "anomaly_ids": answer_ids[:6],   # top-6 (top-3 from each)
            "expected_top3": answer_ids[:3],
            "rationale": "SOG spike (trawl out) at t+1500s; oil pressure dip at t+1800s.",
        },
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["tier0", "tier2"])
    ap.add_argument("--out-dir", default=r"C:\Users\casey\tzpro-personal\training\hermes",
                    help="where to write the seed files")
    args = ap.parse_args()

    out = Path(args.out_dir)
    out.mkdir(parents=True, exist_ok=True)

    if args.mode == "tier0":
        data = seed_tier0()
        mp = out / "tier0_moment.json"
        ap_ = out / "tier0_answer.json"
        with mp.open("w", encoding="utf-8") as f:
            json.dump(data["moment"], f, indent=2)
        with ap_.open("w", encoding="utf-8") as f:
            json.dump(data["answer"], f, indent=2)
        print(f"wrote {mp}")
        print(f"wrote {ap_}")
        print(f"hint for hermes_send.py: --tier 0 --moment {mp}")
    else:
        data = seed_tier2()
        nmea_p = out / "tier2_nmea.jsonl"
        eng_p = out / "tier2_engine.jsonl"
        ans_p = out / "tier2_answer.json"
        with nmea_p.open("w", encoding="utf-8") as f:
            for m in data["nmea"]:
                f.write(json.dumps(m) + "\n")
        with eng_p.open("w", encoding="utf-8") as f:
            for m in data["engine"]:
                f.write(json.dumps(m) + "\n")
        with ans_p.open("w", encoding="utf-8") as f:
            json.dump(data["answer"], f, indent=2)
        print(f"wrote {nmea_p}")
        print(f"wrote {eng_p}")
        print(f"wrote {ans_p}")
        print(f"hint for hermes_send.py:")
        print(f"  --tier 2 --moment-window {nmea_p} --engine-window {eng_p} --answer {ans_p}")
    return 0


if __name__ == "__main__":
    sys.exit(main())