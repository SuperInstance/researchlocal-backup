# Engineer's Reference — TZ Pro Agent

> **Audience:** Engineers, integrators, and successor agents who need a code-grounded
> map of every subsystem in this repository. This is **not** an operator manual
> (see `docs/BOAT_RUNBOOK.md` for that). Every claim here is anchored to a file
> and line range. When the code drifts from the docs, **fix the docs**, not
> the ship.

---

## Scope

This directory contains the engineer's reference set for the `tzpro-agent`
system — a long-running data-collection and analysis node deployed on
the F/V *Eileen* (a salmon troller operating in SE Alaska / Sitka area).

The system:

1. **Captures** GPS / NMEA 0183 from a u-blox GPS receiver on `COM6`.
2. **Relays** NMEA sentences to TimeZero Professional (`TZ Pro`) over TCP.
3. **Captures** a screenshot of the second monitor (DISPLAY6, 1920×1080)
   every 10 minutes on the hour boundary — these are the echograms.
4. **Annotates** each capture with vessel position, SOG/COG, and metadata.
5. **Ingests** capture summaries to Ship Log Search (cloud index).
6. **Surfaces** a Starlette LAN dashboard on port 8090 for wheelhouse status.
7. **Exposes** a tray icon (pystray) on the captain's machine for one-click
   verification and folder access.

---

## Document Map

| File | Subsystem | Read when… |
|---|---|---|
| `00_architecture.md` | System overview | you want the bird's-eye view |
| `01_nmea_bridge.md` | GPS serial → TCP relay | fixing or extending the bridge |
| `02_capture_daemon.md` | TZ Pro lifecycle supervisor | debugging capture uptime |
| `03_capture_v3.md` | Echogram capture worker | changing capture cadence or format |
| `04_tray_app.md` | pystray tray icon | updating the captain-facing UI |
| `05_dashboard.md` | Starlette LAN dashboard | adding API endpoints |
| `06_doctor.md` | Health-check + auto-repair | chasing a `doctor` failure |
| `07_vault.md` | DPAPI secret store | managing API keys safely |
| `08_providers.md` | LLM provider abstraction | swapping model backends |
| `09_schema.md` | Moment / Position / vocabulary | extending the data model |
| `10_cascade.md` | Cascade loops (minutely → daily) | tuning cascade cadence |
| `11_memory.md` | Memory bridge / blobs / recall | wiring search/recall |
| `12_daily_workflow.md` | Operator cadence | onboarding a relief captain |
| `13_troubleshooting.md` | Symptom → fix cookbook | something is on fire |

---

## Reading Order for a New Engineer

1. `00_architecture.md` — mental model
2. `01_nmea_bridge.md` — the load-bearing subsystem
3. `02_capture_daemon.md` + `03_capture_v3.md` — priority-1 (echogram capture)
4. `07_vault.md` — security boundary before touching secrets
5. `08_providers.md` — model routing before touching AI features
6. The rest, on demand

---

## Conventions Used in This Document Set

- **File references** are written `path/to/file.py:LINE` for direct jumps.
  In a code editor with a "go to line" plugin this is one click.
- **Code snippets** are shortened for readability; `...` means ellided.
- **Phase references** map to `docs/phases/` (Phase 1 = capture,
  Phase ∞ = projection / Theia).
- **"Suit vs. Person"** — public knowledge lives in this repo (the *suit*).
  Personal logs, API keys, and the captain's diary live in
  `~/tzpro-personal/` (the *person*). See `docs/SUIT_VS_PERSON.md`.

---

## Repo Coordinates

- **Repo root:** `C:\Users\casey\tzpro-agent\`
- **Remote:** `https://github.com/SuperInstance/tzpro-agent.git`
- **Branch:** `master`
- **Vessel config:** `vessel.json`
- **Runtime state:** `bridge/state.jsonl`, `vessel_state.jsonl`
- **Captures:** `captures/v3/<YYYY-MM-DD>_<lat>N_<lon>W>/<HHMM>_*.{png,json,md}`
- **Vault:** `~/.tzpro-agent/vault.dat` (DPAPI-encrypted, not in repo)

---

## Bootstrapping a Fresh Clone

```bash
# 1. Clone
git clone https://github.com/SuperInstance/tzpro-agent.git
cd tzpro-agent

# 2. Python env
python -m venv .venv
.venv\Scripts\activate
pip install -r requirements.txt   # if it exists; otherwise see 00_architecture.md

# 3. Configure vessel
# Edit vessel.json: vessel name, GPS COM port, timeouts, etc.

# 4. Initialize vault (Windows-only; DPAPI)
python vault.py init

# 5. Smoke-test bridge
python nmea_bridge.py --port COM6 --baud 4800 --diag --diag-seconds 8

# 6. Start bridge for real
python nmea_bridge.py --port COM6 --baud 4800

# 7. Start capture supervisor
python capture_daemon.py run --auto

# 8. (Optional) start dashboard
python dashboard.py

# 9. (Recommended) start tray icon
python tray_app.py
```

---

## License & Ownership

Private repository owned by the vessel operator. See `LICENSE` (if present)
and `docs/SUIT_VS_PERSON.md` for what may be shared externally.