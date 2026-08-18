# Phase 4 — Additional Source Feeds

> **Status:** Planned
> **Depends on:** Phase 1 (moment schema), Phase 2 (analyzer)

## Goal

The platform stops being just a sounder watcher. Every data source on the
boat becomes a moment stream.

## New sources

| Source | Cadence | Transport | Notes |
|---|---|---|---|
| **AIS** | every few seconds | NMEA 0183 via TCP or UDP | vessel name, range, bearing, CPA, TCPA |
| **Radar** | 2-3 sec | NMEA 0183 (TTM, TLL) or proprietary | tracks, ARPA targets, guard zones |
| **Autopilot** | 1 Hz | NMEA 2000 PGNs (127245, 127250) | heading, rudder, mode |
| **Engine gauges** | 1 Hz | NMEA 2000 PGNs (127488, 127489, 127493) | RPM, oil pressure, temp, fuel rate |
| **Depth** | 1 Hz | NMEA 0183 (DPT, DBT) or NMEA 2000 (128267) | already partially captured |
| **Wind** | 1 Hz | NMEA 0183 (MWV) or NMEA 2000 (130306) | already partially captured |
| **GPS** | 1-10 Hz | NMEA 0183 (RMC, GGA) | already captured |

## What's new in Phase 4

- `sources/` package — one module per source, each writes to the
  unified moment schema
- Source auto-discovery via NMEA 0183 multiplex + NMEA 2000 bus scan
- Dashboard filter: "show only echogram" / "show only AIS" / etc.
- Dashboard lanes-per-source view becomes viable (Phase 6 will make this
  a primary view)

## Why this unlocks the bigger vision

Every new source makes the system smarter about the boat's actual state.
Once AIS is flowing, the chatbot can answer "did that seiner cross our
lines yesterday?" Once engine gauges are flowing, "what was our average
RPM while trolling marks at 30 fm?" becomes a trivial query.
