"""schema/correlation.py - Cross-source event recognition.

A Correlation links multiple Anomalies (or Moments) that, taken together,
mean more than any one alone.

Examples from CAPTURE_PIPELINE.md:
  - Depth change + thermal shift + audio change = "we crossed a thermocline"
  - Slow drift + underwater camera anomaly + audio whir = "line on the prop"

Like Anomalies, Correlations are stored as Moments with source="analysis"
so they ride the same storage bus.

This file defines CorrelationRecord (in-process) and the to_moment()
helper that wraps it.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Optional

from .moment import Moment, _new_id, _utcnow_iso


# Canonical correlation types. New types are allowed but these are the
# ones Hermes will be trained on first.
CORR_THERMOCLINE = "thermocline"
CORR_FISH_EVENT = "fish_event"
CORR_MECHANICAL = "mechanical"
CORR_BIOLOGICAL = "biological"     # whales, birds, jellyfish blooms, etc.
CORR_WEATHER = "weather"           # barometric + wind + sea state


@dataclass
class CorrelationRecord:
    """A pattern across multiple Anomalies/Moments.

    Attributes
    ----------
    moment_ids : list[str]
        The participating Moments/Anomalies. Order doesn't matter.
    correlation_type : str
        One of CORR_* above, or a new type with a clear meaning.
    confidence : float
        0.0 to 1.0. Hermes/the analyzer's confidence this is real.
    narrative : str | None
        Plain-English explanation. Hermes drafts these; the captain
        reads them. Short - one or two sentences.
    evidence : dict
        Free-form per-type evidence. E.g. for "thermocline":
            {"depth_delta_m": 8.5, "thermal_delta_c": 2.1,
             "audio_change": "hull-pop absent"}
    span_seconds : float | None
        Time window across which the participating Moments occurred.
        Helps Hermes learn what "concurrent" means per event type.
    """
    moment_ids: list[str]
    correlation_type: str
    confidence: float = 0.5
    narrative: Optional[str] = None
    evidence: dict[str, Any] = field(default_factory=dict)
    span_seconds: Optional[float] = None

    def to_moment(self, importance: float = 0.8) -> Moment:
        if not self.moment_ids:
            raise ValueError("CorrelationRecord needs at least one moment_id")
        # Use the earliest of the participating timestamps as the correlation's
        # timestamp, falling back to now if we don't have them.
        ts = _utcnow_iso()
        return Moment(
            id=_new_id(),
            timestamp=ts,
            source="analysis",
            payload={
                "kind": "correlation",
                "correlation_type": self.correlation_type,
                "participating_moment_ids": list(self.moment_ids),
                "confidence": self.confidence,
                "narrative": self.narrative,
                "evidence": dict(self.evidence),
                "span_seconds": self.span_seconds,
            },
            tags=[f"corr:{self.correlation_type}"],
            importance=importance,
            # No single parent - correlations are N-ary.
            parent_id=None,
        )