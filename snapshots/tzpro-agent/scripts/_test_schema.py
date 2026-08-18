"""scripts/_test_schema.py

Smoke test for schema/moment.py + schema/anomaly.py + schema/correlation.py.

Verifies:
  1. Moment round-trips through to_dict / from_dict / to_json / from_json
  2. Position handles missing lat/lon (nullable, not zero-defaulted)
  3. AnomalyRecord.to_moment() produces a valid Moment with source='analysis'
  4. AnomalyRecord rejects invalid severity
  5. CorrelationRecord.to_moment() wraps N participating moments
  6. JSON is the truth: markdown_render() never adds data, only formats
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Make the repo root importable when running from anywhere
REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from schema import (
    Moment,
    Position,
    AnomalyRecord,
    CorrelationRecord,
    SEVERITY_ALARM,
    SEVERITY_INFO,
    SEVERITY_CRITICAL,
    SEVERITY_WARNING,
    CORR_THERMOCLINE,
)


def t(test_name: str, ok: bool, detail: str = "") -> bool:
    marker = "[ok]  " if ok else "[FAIL]"
    print(f"{marker} {test_name}{(' - ' + detail) if detail else ''}")
    return ok


def test_moment_roundtrip() -> bool:
    m = Moment(
        source="sounder",
        position=Position(lat=55.7833, lon=-132.4167, sog_knots=4.2, cog_deg=270),
        payload={"path": "/captures/2026-07-24/1230.png", "width": 1920, "height": 1080},
        tags=["fish_event"],
        importance=0.7,
    )
    as_json = m.to_json()
    m2 = Moment.from_json(as_json)
    m3 = Moment.from_dict(m.to_dict())
    return t(
        "Moment JSON round-trip",
        m2.to_dict() == m.to_dict() and m3.to_dict() == m.to_dict(),
        f"id={m.id}",
    )


def test_position_nullable() -> bool:
    p = Position()
    m = Moment(source="voice", position=p, payload={"text": "got one!"})
    d = m.to_dict()
    # null lat/lon must be None, not 0.0 (0 is Gulf of Guinea)
    return t(
        "Position nullable preserves None (not 0.0)",
        d["position"]["lat"] is None and d["position"]["lon"] is None,
        f"lat={d['position']['lat']}, lon={d['position']['lon']}",
    )


def test_anomaly_to_moment() -> bool:
    a = AnomalyRecord(
        moment_id="m_test_001",
        source="engine",
        baseline={"oil_pressure_psi": 60.0, "oil_temp_c": 85.0},
        observed={"oil_pressure_psi": 42.0, "oil_temp_c": 102.0},
        delta_pct=-30.0,
        severity=SEVERITY_ALARM,
        confidence=0.82,
        recommended_action="reduce RPM, check oil level",
        context={"m_neighbor_1": "before", "m_neighbor_2": "after"},
    )
    m = a.to_moment()
    return t(
        "AnomalyRecord wraps as source='analysis' Moment",
        (
            m.source == "analysis"
            and m.parent_id == "m_test_001"
            and m.payload["kind"] == "anomaly"
            and m.payload["severity"] == SEVERITY_ALARM
            and "severity:alarm" in m.tags
            and m.importance >= 0.8
        ),
        f"importance={m.importance:.2f}",
    )


def test_anomaly_severity_validation() -> bool:
    a = AnomalyRecord(moment_id="m_x", source="thermal", severity="bogus")
    try:
        a.to_moment()
        return t("AnomalyRecord rejects invalid severity", False, "no exception")
    except ValueError:
        return t("AnomalyRecord rejects invalid severity", True)


def test_correlation_to_moment() -> bool:
    c = CorrelationRecord(
        moment_ids=["m_001", "m_002", "m_003"],
        correlation_type=CORR_THERMOCLINE,
        confidence=0.75,
        narrative="Depth, thermal, and audio all changed within 90s - crossed thermocline.",
        evidence={"depth_delta_m": 8.5, "thermal_delta_c": 2.1},
        span_seconds=90.0,
    )
    m = c.to_moment()
    return t(
        "CorrelationRecord wraps N-ary as analysis Moment",
        (
            m.source == "analysis"
            and m.payload["kind"] == "correlation"
            and m.payload["correlation_type"] == CORR_THERMOCLINE
            and len(m.payload["participating_moment_ids"]) == 3
            and m.parent_id is None
            and "corr:thermocline" in m.tags
        ),
    )


def test_correlation_requires_participants() -> bool:
    c = CorrelationRecord(moment_ids=[], correlation_type=CORR_THERMOCLINE)
    try:
        c.to_moment()
        return t("CorrelationRecord rejects empty moment_ids", False)
    except ValueError:
        return t("CorrelationRecord rejects empty moment_ids", True)


def test_markdown_render_is_format_only() -> bool:
    """markdown_render() must not invent data; to_json() must round-trip
    even after rendering to markdown.
    """
    m = Moment(source="sounder", position=Position(lat=55.0, lon=-132.0), payload={"path": "/x.png"})
    md_before = m.markdown_render()
    m2 = Moment.from_json(m.to_json())
    md_after = m2.markdown_render()
    # Both should contain the id and source; data dict unchanged
    return t(
        "markdown_render is f(JSON), not the source of truth",
        m.id in md_before and m.id in md_after and m.to_dict() == m2.to_dict(),
    )


def main() -> int:
    tests = [
        test_moment_roundtrip,
        test_position_nullable,
        test_anomaly_to_moment,
        test_anomaly_severity_validation,
        test_correlation_to_moment,
        test_correlation_requires_participants,
        test_markdown_render_is_format_only,
    ]
    results = [fn() for fn in tests]
    print(f"\n{'='*40}\n{sum(results)}/{len(results)} passed")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())