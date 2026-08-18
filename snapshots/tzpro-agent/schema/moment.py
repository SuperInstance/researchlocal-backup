"""schema/moment.py - The atomic unit of capture.

Every sensor on the boat emits a Moment. The Moment schema is the
universal shape that lets sounder screenshots, NMEA fixes, engine
telemetry, thermal images, audio clips, and voice notes all flow
through one queryable stream.

This file is in the repo (the "suit"). The data that fills these
schemas lives in ~/tzpro-personal/ (the "person"). See
docs/architecture/CAPTURE_PIPELINE.md and docs/SUIT_VS_PERSON.md.

Design principle: schema is stable, payload is open. Every Moment
has the same envelope (timestamp + position + source + payload).
The payload itself is a free-form dict whose keys are per-source.
That way new sensors can be added without schema migrations.

JSON truth: every Moment serializes deterministically via to_dict().
The companion from_dict() parses. markdown_render() is a *separate*
function that produces human-readable text - it's f(JSON), never
the source of truth.
"""
from __future__ import annotations

from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from typing import Any, Optional
import json
import uuid


def _utcnow_iso() -> str:
    """ISO-8601 in UTC, second precision. Stable across serializations."""
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def _new_id() -> str:
    """Short, sortable, unique-ish ID. Uses timestamp + uuid4 suffix."""
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
    return f"m_{ts}_{uuid.uuid4().hex[:8]}"


@dataclass
class Position:
    """Where the boat was when this Moment was captured.

    Decimal degrees. Optional because some sources (e.g. a voice note
    recorded at the dock) may not have a position fix. Position is
    nullable, not zero-defaulted - zero is a real place (Gulf of Guinea).
    """
    lat: Optional[float] = None
    lon: Optional[float] = None
    sog_knots: Optional[float] = None   # speed over ground
    cog_deg: Optional[float] = None     # course over ground (0-360)
    fix_quality: str = "unknown"        # "gps" | "dgps" | "estimated" | "none" | "unknown"


@dataclass
class Moment:
    """The atomic unit of capture. Every sensor emits into this shape.

    Attributes
    ----------
    id : str
        Globally unique. Format: m_YYYYMMDDTHHMMSS_<8-hex>.
    timestamp : str
        ISO-8601 UTC, second precision. Always UTC, never local.
    source : str
        Which sensor emitted this. Free-form but conventionally:
        - "sounder"        - 10-min boundary-aligned TZ Pro screenshot
        - "nmea"           - parsed NMEA sentence (GPGGA, GPRMC, etc.)
        - "engine"         - NMEA 2000 PGN (RPM, oil pressure, etc.)
        - "thermal"        - thermal camera frame
        - "audio"          - audio clip or audio stats
        - "voice"          - captain voice note (after STT)
        - "analysis"       - model-produced interpretation of another Moment
    position : Position
        Boat position at this timestamp. Optional.
    payload : dict
        Free-form per-source data. Schema is per-source:
        - sounder: {path: str, width: int, height: int, hash: str}
        - nmea:    {sentence: str, parsed: dict}
        - engine:  {pgn: int, fields: dict}
        - thermal: {path: str, min_c: float, max_c: float, mean_c: float}
        - audio:   {path: str | None, rms: float, peak: float, centroid_hz: float}
        - voice:   {text: str, stt_model: str, duration_s: float}
        - analysis:{model: str, prompt: str, response: str, confidence: float,
                    inputs: list[str]}  # moment_ids analyzed
    tags : list[str]
        Free-form labels (e.g. "thermocline", "fish_event", "fault").
    importance : float
        0.0 to 1.0. Set by the source (anomalies start at 0.8+,
        routine telemetry at 0.1-0.3). The analyzer can raise it.
    parent_id : str | None
        If this Moment was *produced* by analyzing another (e.g.
        an "analysis" Moment has a parent "sounder" Moment), link them.
    """
    id: str = field(default_factory=_new_id)
    timestamp: str = field(default_factory=_utcnow_iso)
    source: str = ""
    position: Position = field(default_factory=Position)
    payload: dict[str, Any] = field(default_factory=dict)
    tags: list[str] = field(default_factory=list)
    importance: float = 0.5
    parent_id: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        """Deterministic JSON-serializable dict. JSON is the truth."""
        d = asdict(self)
        return d

    def to_json(self) -> str:
        """Compact JSON string. Stable key order via dataclass."""
        return json.dumps(self.to_dict(), separators=(",", ":"), sort_keys=True)

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Moment":
        """Parse from dict. Tolerates missing fields with defaults."""
        pos = d.get("position") or {}
        if isinstance(pos, dict):
            pos = Position(**{k: v for k, v in pos.items()
                              if k in Position.__dataclass_fields__})
        else:
            pos = Position()
        kwargs = {
            "id": d.get("id") or _new_id(),
            "timestamp": d.get("timestamp") or _utcnow_iso(),
            "source": d.get("source", ""),
            "position": pos,
            "payload": dict(d.get("payload") or {}),
            "tags": list(d.get("tags") or []),
            "importance": float(d.get("importance", 0.5)),
            "parent_id": d.get("parent_id"),
        }
        return cls(**kwargs)

    @classmethod
    def from_json(cls, s: str) -> "Moment":
        return cls.from_dict(json.loads(s))

    def markdown_render(self) -> str:
        """Human-readable render. NEVER the source of truth - just f(self).

        Used by dashboard, daily plans, hermes training reports.
        """
        pos = self.position
        pos_str = "n/a"
        if pos.lat is not None and pos.lon is not None:
            pos_str = f"{pos.lat:.4f}, {pos.lon:.4f}"
            if pos.sog_knots is not None:
                pos_str += f" @ {pos.sog_knots:.1f}kt"

        lines = [
            f"### Moment {self.id}",
            f"- **Time:** {self.timestamp}",
            f"- **Source:** {self.source}",
            f"- **Position:** {pos_str}",
            f"- **Importance:** {self.importance:.2f}",
        ]
        if self.tags:
            lines.append(f"- **Tags:** {', '.join(self.tags)}")
        if self.parent_id:
            lines.append(f"- **Parent:** `{self.parent_id}`")
        lines.append("- **Payload:**")
        if self.payload:
            for k, v in self.payload.items():
                v_str = json.dumps(v) if not isinstance(v, str) else v
                if len(v_str) > 200:
                    v_str = v_str[:200] + "..."
                lines.append(f"  - `{k}`: {v_str}")
        else:
            lines.append("  _(empty)_")
        return "\n".join(lines)