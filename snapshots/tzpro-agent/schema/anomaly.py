"""schema/anomaly.py - A deviation from the learned baseline.

The capture pipeline emits Moments. The analyzer reads Moments and
emits Anomalies. An anomaly is the analyzer's *judgment* that this
Moment is worth the captain's attention.

Anomalies are themselves stored as Moments with source="analysis"
and parent_id pointing at the triggering Moment. That keeps the
data model uniform - everything is a Moment.

This file defines the analysis-side data structure (the fields the
detector writes into the anomaly's payload) plus a thin helper to
construct the wrapping Moment.

JSON truth: the wrapping Moment IS the storage form. AnomalyRecord
below is the in-process dataclass for the analyzer's internal use
before serialization.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any, Optional

from .moment import Moment, _new_id, _utcnow_iso


SEVERITY_INFO = "info"           # notable, no action needed
SEVERITY_WARNING = "warning"     # worth knowing, maybe investigate
SEVERITY_ALARM = "alarm"         # captain should look now
SEVERITY_CRITICAL = "critical"   # safety/equipment - act now

VALID_SEVERITIES = {
    SEVERITY_INFO,
    SEVERITY_WARNING,
    SEVERITY_ALARM,
    SEVERITY_CRITICAL,
}


@dataclass
class AnomalyRecord:
    """A deviation from the learned baseline.

    Attributes
    ----------
    moment_id : str
        ID of the triggering Moment. Required.
    timestamp : str
        When the anomaly was detected (ISO-8601 UTC).
    source : str
        Which sensor the anomaly came from (e.g. "engine", "thermal").
        Mirrors the source of the triggering Moment.
    baseline : dict
        What "normal" looked like at detection time. Per-source keys.
    observed : dict
        What we actually saw. Same keys as baseline, so delta is computable.
    delta_pct : float | None
        Optional pre-computed percent change for the most relevant field.
        Detector's choice which field to highlight.
    severity : str
        One of SEVERITY_INFO/WARNING/ALARM/CRITICAL.
    confidence : float
        0.0 to 1.0. Detector's confidence this is a real anomaly, not noise.
    recommended_action : str | None
        Plain-English suggestion ("check port-side exhaust", "reduce RPM").
        Free-form; the dashboard renders as-is.
    context : dict
        Surrounding Moments (the +/- N around the trigger). Helps
        Hermes learn what context the captain cares about.
    """
    moment_id: str
    timestamp: str = field(default_factory=_utcnow_iso)
    source: str = ""
    baseline: dict[str, Any] = field(default_factory=dict)
    observed: dict[str, Any] = field(default_factory=dict)
    delta_pct: Optional[float] = None
    severity: str = SEVERITY_INFO
    confidence: float = 0.5
    recommended_action: Optional[str] = None
    context: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "moment_id": self.moment_id,
            "timestamp": self.timestamp,
            "source": self.source,
            "baseline": dict(self.baseline),
            "observed": dict(self.observed),
            "delta_pct": self.delta_pct,
            "severity": self.severity,
            "confidence": self.confidence,
            "recommended_action": self.recommended_action,
            "context": dict(self.context),
        }

    def to_moment(self, importance: Optional[float] = None) -> Moment:
        """Wrap as a Moment with source='analysis' so it flows through
        the same storage and query paths as everything else.
        """
        if self.severity not in VALID_SEVERITIES:
            raise ValueError(f"invalid severity: {self.severity!r}")
        if importance is None:
            importance = {
                SEVERITY_INFO: 0.4,
                SEVERITY_WARNING: 0.6,
                SEVERITY_ALARM: 0.85,
                SEVERITY_CRITICAL: 0.95,
            }.get(self.severity, 0.5)

        return Moment(
            id=_new_id(),
            timestamp=self.timestamp,
            source="analysis",
            payload={
                "kind": "anomaly",
                "trigger_moment_id": self.moment_id,
                "trigger_source": self.source,
                "baseline": self.baseline,
                "observed": self.observed,
                "delta_pct": self.delta_pct,
                "severity": self.severity,
                "confidence": self.confidence,
                "recommended_action": self.recommended_action,
                "context_moment_ids": list(self.context.keys()),
            },
            tags=[f"severity:{self.severity}", f"source:{self.source}"],
            importance=importance,
            parent_id=self.moment_id,
        )