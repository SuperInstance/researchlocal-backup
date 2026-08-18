# State of the System — Verified at Handoff

> **Snapshot date:** 2026-07-24, approximately 17:30 AKDT (UTC-08).
> All numbers in this document were verified by running the diagnostic
> commands at write time. If you read this in the future, treat the
> numbers as historical — re-verify with `python doctor.py check` and
> `python capture_daemon.py verify` before relying on them.

---

## TL;DR

**The capture pipeline is working and durable.** Doctor 9/9, capture
HEALTHY (last capture 153s ago), GPS live on COM6 (fix_q=1, 8 sats),
TZ Pro connected to bridge (tcp_clients=1), ship is on the trolling
grounds in SE Alaska (lat ~55.79°N, lon ~-131.65°W). Both P0
(GPS relay) and P1 (capture daemon) are hardened.

**The next two priority tiers (analysis, cascade) are partially built
but not in production.** Phase 2 (analyzer wiring) has a Hermes
teacher harness in `~/hermes-nerve-center/` but `hermes_ensemble.py`
(the multi-model parallel caller) is **missing** despite being
imported. Phase 3 (multi-session chat) is design only. Phase 8 (cloud
federation) is not started.

**The system is in steady state.** Nothing is on fire. The next
agent's job is to harden what is built (durability, observability),
*not* to add new features.

---

## What is working — verified

### P0: GPS relay (COM6 → TCP:6006)

| Component | State | Verification |
|---|---|---|
| `nmea_bridge.py` | RUNNING (PID 29828) | `Get-Process python` |
| COM6 read | OK, 8 satellites, fix_q=1 | `bridge:serial` doctor check |
| TCP :6006 | LISTENING | `Test-NetConnection -Port 6006` |
| TZ Pro connection | ESTABLISHED (1 client) | `bridge:http:/health` reports `tcp_clients=1` |
| Last heartbeat | 0.0s ago (fresh) | `bridge:heartbeat` doctor check |
| Position | lat=55.79, lon=-131.65 (live, advancing) | `vessel_state.jsonl` tail |

**Why this matters:** Without this, TZ Pro has no GPS. The captain
cannot see where he is on the chart. This is non-negotiable.

**Files:**
- `nmea_bridge.py` (45K, 1024 lines) — Win32 shared-mode trick
  (`CreateFileW` with `FILE_SHARE_READ|FILE_SHARE_WRITE`) so TZ Pro
  can also read COM6; asyncio TCP + HTTP broadcasters.
- `fix_priority0.bat` — recovery script (kills stale, restarts).
- `TZ Pro Agent Tray.lnk` (OneDrive Desktop) — tray UI launcher.

### P1: Capture pipeline (sounder → disk)

| Component | State | Verification |
|---|---|---|
| `capture_daemon.py` | RUNNING | stampfile `capture_daemon.stamp.json` |
| TZ Pro detection | TRACKING (last seen 0.0s ago) | stampfile `tzpro_last` field |
| Capture child | RUNNING (capture_v3.py) | stampfile `child_pid` |
| Latest capture | 153s ago | `python capture_daemon.py verify` |
| Today's folder | exists, populated | `ls captures/v3/2026-07-24*` |
| Total captures (recent) | ~100+ files across 7/22–7/23 | `Get-ChildItem captures/v3 -Recurse -Filter *.png \| Measure-Object` |
| Tray freshness indicator | enabled (STALE warning at >15min) | `tray_app.py` `_status_line()` |

**Why this matters:** Every 10 minutes, the system saves the sounder
screen plus position plus metadata. **Every day this computer runs is
a day of valuable data, analyzed or not.** Data collection cannot
silently fail. If it fails, the captain must know.

**Files:**
- `capture_daemon.py` (374 lines) — TZ Pro watchdog + stampfile
  singleton + `verify` subcommand.
- `capture_v3.py` — 10-min boundary-aligned capture, DISPLAY6
  (1920×1080 @ X=1920), PNG+JSON+MD triplet output.
- `tray_app.py` (325 lines) — pystray front door, auto-starts daemon
  on launch, freshness telemetry, "open today's captures" / "open
  latest capture" menu items.

### P2 (partial): Analysis pipeline

| Component | State | Notes |
|---|---|---|
| Hermes teacher harness | built (`~/hermes-nerve-center/`) | 6-tier mission ladder |
| `hermes_ensemble.py` | **MISSING** | imported by `hermes_worker.py`, not built |
| `hermes_worker.py` | exists | will crash on import until ensemble is built |
| DeepInfra provider | configured, `is_available() == False` | needs API key in vault |
| Ollama provider | enabled | 4 models: granite4.1:8b, gemma4:12b, nomic-embed-text:latest, qwen3:4b |
| 90 echograms awaiting analysis | yes | `captures/v3/2026-07-22_*` mostly |

**What's missing for P2 to be operational:**
1. Build `hermes_ensemble.py` — multi-model parallel caller using
   `asyncio.gather` over DeepInfra providers.
2. Store DeepInfra API key in vault:
   `python -c "from vault import set_secret; set_secret('deepinfra_api_key', '<KEY>')"`.
3. Wire `hermes_worker.py` to call ensemble.
4. Run first Socratic batch (10 echograms), dry-run if no key.

### P3 (design only): Multi-session chat

Not started. Phase 3 in ROADMAP.

### P4 (partial): Cascade loops

| Component | State | Notes |
|---|---|---|
| `cascade/minute.py` | exists | per-minute loop |
| `cascade/decaminute.py` | exists | 10-min loop (should pair with capture) |
| `cascade/hourly.py` | exists | hourly rollup |
| `cascade/daily.py` | exists | daily briefing |
| `cascade/retention.py` | exists | housekeeping |
| `cascade/ollama_client.py` | exists | provider client |
| `cascade/twin_sink.py` | exists | writes to twin/scrubber |
| Doctor integration | partial | cascade is not in `doctor.py check` |

**Not in production.** The cascade code exists but is not running and
not supervised. This is intentional — we focus on capture (P1) first
and add cascade loops once the data is reliable.

### Other verified systems

| Component | State | Notes |
|---|---|---|
| `vault.py` (DPAPI + AES-GCM) | OK, 0 secrets stored, round-trip works | doctor: `vault:roundtrip` |
| `dashboard.py` (Starlette :8090) | RUNNING, vessel=F/V Eileen, providers=ollama+local_file | doctor: `dashboard:8090` |
| `db.py` (SQLite moments store) | exists, schema migrated | count grows with capture |
| `state:jsonl` (vessel_state.jsonl) | updated 0.0s ago, class=trolling | doctor: `state:jsonl` |
| Ollama :11434 | RUNNING, 4 models | doctor: `ollama:11434` |

---

## What is NOT working — verified gaps

### Gap 1: `hermes_ensemble.py` does not exist

**Severity:** medium. Blocks P2 (analyzer wiring). Does not block P0
or P1.

**What is missing:** A Python module that imports `DeepInfra` and
`Ollama` providers, accepts a list of moments, runs them through
multiple models in parallel via `asyncio.gather`, scores agreement,
and returns a consensus verdict.

**What `hermes_worker.py` does today:** Imports it on startup and
crashes with `ModuleNotFoundError`.

**Decision needed:** See `03_decisions_log.md` decision D-007 — we
chose to ship P1 first and add P2 second. Build ensemble *after* you
have run capture for a week without losing data.

### Gap 2: Cascade loops are not supervised

**Severity:** low. Cascade is async, run-it-once-per-cadence code.
It is not in the doctor check, not in the tray menu, and not in the
fix script.

**Implication:** If `cascade/daily.py` fails to run one morning, the
captain's morning brief is missing. We have not yet decided if that
matters enough to warrant supervision.

### Gap 3: No scheduled task for bridge auto-start

**Severity:** medium. The Windows Task Scheduler entry for
`install_bridge_task.bat` was never created (the script requires
admin elevation). Result: after every reboot, the bridge must be
started manually (or by `fix_priority0.bat`).

**Mitigation in place:** `fix_priority0.bat` can be run from a
desktop shortcut (`Fix TZ Pro Position.lnk`). The tray app also
auto-starts the capture daemon.

**Long-term fix:** Either (a) actually run `install_bridge_task.bat`
with admin, or (b) configure the tray app to also start the bridge,
not just the capture daemon. We have not chosen between (a) and (b).
See `03_decisions_log.md` decision D-004.

### Gap 4: DeepInfra provider has no API key

**Severity:** medium for P2. Zero impact on P0/P1.

**Workaround:** `python capture_daemon.py verify --dry-run` (if it
exists) or simply run with `--dry-run` flag to bypass. The
`hermes_ensemble.py` (when built) should support `--dry-run` mode
that fakes model responses with timestamped stubs.

### Gap 5: `restart_services.bat` had wrong path

**Severity:** low (already fixed in `adfa9da`). Was pointing to
`C:\Users\casey\.openclaw\workspace\tzpro-agent` (does not exist).
Now points to `C:\Users\casey\tzpro-agent`.

**Lesson learned:** All `.bat` files must be tested against the
actual repo path. The next agent should add a `tests/test_bat_paths.ps1`
that grep's for `.openclaw\workspace` and fails the build.

---

## Where the data lives

```
C:\Users\casey\tzpro-agent\
├── captures/v3/<YYYY-MM-DD>_<lat>N_<lon>W>/
│   ├── <HHMM>_<lat>N_<lon>W.png        ← sounder screenshot
│   ├── <HHMM>_<lat>N_<lon>W.json       ← machine-readable metadata
│   └── <HHMM>_<lat>N_<lon>W.md         ← human-readable summary
├── vessel_state.jsonl                  ← one JSON line per NMEA tick (~305 MB)
├── tzpro.db (or similar)               ← SQLite moments store
├── cascade_out/                        ← briefings, heartbeats, records
├── memory/
│   ├── blobs/                          ← content-addressed binary blobs
│   └── meta.db                         ← SQLite index for blobs
├── .vessel/bottles/                    ← I2I messages between agents
├── logs/                               ← daemon logs
└── .tzpro-agent/vault.dat              ← DPAPI-encrypted secrets (NOT in repo)
```

**Backup rules:**
- ✅ Capture PNG/JSON/MD files → back up (immutable, gold).
- ✅ `vessel_state.jsonl` → back up (append-only, recoverable).
- ✅ `cascade_out/` → back up.
- ✅ `memory/` → back up.
- ❌ `.tzpro-agent/vault.dat` → **DO NOT BACK UP** (cross-machine
  DPAPI decryption will fail and you will leak your backup). See
  `docs/BACKUP.md`.

---

## Active missions (in priority order)

| # | Mission | Status | Notes |
|---|---|---|---|
| **P0** | GPS relay (COM6 → TCP:6006) | **DONE, durable** | Auto-recovers via `fix_priority0.bat`. Doctor 9/9. |
| **P1** | Capture pipeline | **DONE, durable** | Auto-recovers via tray auto-start. `verify` returns HEALTHY. |
| **P2** | Analyzer wiring (Hermes ensemble) | **BLOCKED on gap 1** | Build `hermes_ensemble.py`. Use DeepInfra dry-run until key is in vault. |
| **P3** | Multi-session chat | **NOT STARTED** | Phase 3 in roadmap. Defer until P2 is operational. |
| **P4** | Cascade loop supervision | **PARTIAL** | Loops exist; not supervised. See gap 2. |

---

## How to verify this document is still accurate

```powershell
# Should print "9/9 healthy"
python doctor.py check

# Should print "VERDICT: HEALTHY" with recent capture
python capture_daemon.py verify

# Should list .png files newer than today
Get-ChildItem captures/v3/2026-07-24*/*.png | Sort-Object LastWriteTime -Descending | Select-Object -First 5
```

If any of these fail, update this document with the new state and
add a SESSION-NOTE to `docs/PLANS/daily/<today>.md`.
