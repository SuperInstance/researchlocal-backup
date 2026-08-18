"""schema/__init__.py - the canonical shapes of the boat-agent platform.

Every sensor emits a Moment. The analyzer reads Moments and emits
Anomalies (wrapped as Moments). The correlator reads Anomalies and
emits Correlations (also wrapped as Moments). Everything is a Moment.

This package is in the repo (the "suit"). Data flows through these
shapes into ~/tzpro-personal/ (the "person").
"""
from .moment import Moment, Position, _utcnow_iso, _new_id
from .anomaly import (
    AnomalyRecord,
    SEVERITY_INFO,
    SEVERITY_WARNING,
    SEVERITY_ALARM,
    SEVERITY_CRITICAL,
    VALID_SEVERITIES,
)
from .correlation import (
    CorrelationRecord,
    CORR_THERMOCLINE,
    CORR_FISH_EVENT,
    CORR_MECHANICAL,
    CORR_BIOLOGICAL,
    CORR_WEATHER,
)

__all__ = [
    "Moment",
    "Position",
    "AnomalyRecord",
    "CorrelationRecord",
    "SEVERITY_INFO",
    "SEVERITY_WARNING",
    "SEVERITY_ALARM",
    "SEVERITY_CRITICAL",
    "VALID_SEVERITIES",
    "CORR_THERMOCLINE",
    "CORR_FISH_EVENT",
    "CORR_MECHANICAL",
    "CORR_BIOLOGICAL",
    "CORR_WEATHER",
]