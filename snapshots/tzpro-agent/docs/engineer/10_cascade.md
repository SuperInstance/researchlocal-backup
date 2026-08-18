# Cascade — `cascade/`

**Package:** `cascade/`
**Priority:** Phase 2 (in development).
**Owner:** tzpro-agent.

## What It Does

The **cascade** is the periodic analysis loop that fans data out
across providers at multiple cadences. Each cadence layer sees the
last layer's output as its input — like water falling down a cascade.

```
            ┌────────────────┐
            │  raw capture   │  (from capture_v3 + nmea_bridge)
            └───────┬────────┘
                    │
                    ▼
            ┌────────────────┐
            │ minute_loop    │  every 60 s: hot path, local ollama
            └───────┬────────┘
                    │
                    ▼
            ┌────────────────┐
            │ decaminute_loop│  every 10 min: aligned with capture
            └───────┬────────┘
                    │
                    ▼
            ┌────────────────┐
            │ hourly_loop    │  every 60 min: medium-cadence summaries
            └───────┬────────┘
                    │
                    ▼
            ┌────────────────┐
            │ daily_loop     │  once per day: long-horizon analysis
            └────────────────┘
```

Each loop produces **briefings** (markdown) and **records** (JSONL
of Moments) into `cascade_out/`.

## Module Map

| Module | Cadence | Purpose |
|---|---|---|
| `cascade/daemon.py` | — | Supervisor that runs all loops as threads |
| `cascade/minute_loop.py` | 60 s | Local ollama quick checks (anomaly score) |
| `cascade/decaminute_loop.py` | 10 min | Aligned with capture_v3 boundary; per-capture analysis |
| `cascade/hourly_loop.py` | 60 min | Hour-rolling summaries |
| `cascade/daily_loop.py` | 24 h | Once per day at 04:00 AKDT |
| `cascade/config.py` | — | Loops' config (thresholds, prompt templates) |
| `cascade/retention.py` | — | Garbage-collects old cascade output |
| `cascade/roster.py` | — | Per-shift prompt/persona scheduling |
| `cascade/twin_sink.py` | — | Writes cascade output to memory/twin |
| `cascade/notify.py` | — | Sends notifications (Telegram, etc.) |
| `cascade/agents/` | — | Per-role agent definitions (e.g. `analyst.md`) |
| `cascade/tools/` | — | Tools exposed to cascade agents |

## Outputs (`cascade_out/`)

```
cascade_out/
├── briefings/
│   ├── 2026-07-24_minute.md
│   ├── 2026-07-24_decaminute.md
│   └── …
├── heartbeats/
│   └── heartbeat.jsonl
├── logs/
│   └── cascade.log
└── records/
    └── 2026-07-24.jsonl     # one Moment per line
```

## Configuration (`cascade/config.py`)

Defines:

- Loop intervals (each loop reads its own constant)
- Provider chains per task (e.g. `anomaly_score → ollama → deepinfra`)
- Prompt templates (referenced by path under `cascade/agents/`)
- Importance thresholds (`min_importance_to_notify = 0.8`)

## Running

```bash
# One-shot: run each loop once for testing
python -m cascade.minute_loop --once
python -m cascade.decaminute_loop --once
python -m cascade.hourly_loop --once
python -m cascade.daily_loop --once

# Long-lived:
python -m cascade.daemon
```

`cascade/daemon.py` is the supervisor — it spawns each loop in a
thread with appropriate cadence, with graceful shutdown on SIGTERM.

## Design Notes

- **Local first, cloud fallback.** Minute loop uses ollama (free,
  fast, offline). Only when a task exceeds ollama's capability does
  the cascade escalate to deepinfra/openai/etc.
- **Importance gating.** Each Moment has an `importance` score
  (`schema/moment.py`). Only Moments above a threshold flow into
  the next cascade level. This bounds cost and keeps the loops
  focused on signal, not noise.
- **Atomic writes.** Each cascade output uses `.tmp` → rename so
  readers never see half-written files.
- **Tunable cadence.** All loops honor their config constants; if
  a loop is too noisy, raise the threshold or lengthen the interval.

## Status (as of `b64c2ef`)

- `daemon.py`, `daily_loop.py`, `hourly_loop.py`, `decaminute_loop.py`,
  `minute_loop.py` all scaffolded.
- `config.py`, `retention.py`, `roster.py`, `twin_sink.py`, `notify.py`
  present.
- `cascade/README.md` is the authoritative phase-2 doc.

## Verified Line Numbers

- Module map: see `ls cascade/`.
- Loop supervisors: each `*_loop.py` has a `def run_forever()` and
  `if __name__ == "__main__"` entrypoint.

## Related Docs

- `cascade/README.md` — authoritative cascade doc
- `docs/phases/2.md` — phase-2 cascade requirements
- `docs/engineer/08_providers.md` — the providers the cascade uses
- `docs/engineer/09_schema.md` — Moments are the cascade's currency