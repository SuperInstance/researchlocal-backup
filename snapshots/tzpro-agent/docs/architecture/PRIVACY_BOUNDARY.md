# Privacy Boundary — Operational Specification

> **Companion to `docs/SUIT_VS_PERSON.md`.** That document is the
> charter — the *why* and the *what*. This document is the
> specification — the *exactly where* and the *exactly how*.
>
> If you are an agent implementing a new feature, you read this
> file to know where the data goes. If you are a captain onboarding
> to the suit, you read this file to know where to put your data.

---

## TL;DR — the file system layout

```
~/                                                     # user's home
│
├── tzpro-agent/                                       # the SUIT (git repo)
│   ├── docs/
│   ├── delegation/
│   ├── capture_daemon.py
│   ├── doctor.py
│   ├── dashboard.py
│   ├── tray_app.py
│   ├── nmea_bridge.py
│   ├── providers/
│   ├── vault.py                # mechanism (NOT the vault data)
│   ├── vessel_config.py        # defaults (NOT the overrides)
│   ├── schema/
│   ├── cascade/
│   ├── scripts/
│   ├── tests/
│   │   └── fixtures/           # synthetic data only
│   ├── .gitignore              # aggressive exclusion
│   └── README.md
│
└── tzpro-personal/                                    # the PERSON (not a repo)
    ├── vessel_state.jsonl      # NMEA stream append-log
    ├── vessel.db               # SQLite indexed store
    ├── captures/               # dated folders of screenshots
    │   └── 2026-07-23/
    │       ├── captures_v3_*.png
    │       └── decaminute_*.json
    ├── voice/                  # voice notes & transcripts
    │   ├── raw/
    │   └── transcripts/
    ├── logs/                   # runtime logs (capture daemon, bridge, etc.)
    ├── vault.dpapi             # DPAPI-encrypted credentials
    ├── vessel_config.local.json
    ├── lora/                   # trained LoRA weights
    ├── sessions/               # session exports, BATON_PASS archives
    ├── exports/                # anything exported for sharing
    └── daily_log/              # captain's daily notes
```

The boundary is the parent directory. The suit and the person are
siblings. The suit talks to the person via paths in `vessel_config.local.json`.

---

## The .gitignore (aggressive exclusion)

The suit's `.gitignore` is the first line of defense. It must exclude
*every file that could contain personal data* by default. Code is
whitelisted. Data is excluded.

```gitignore
# ──────────────────────────────────────────────
# Personal data (lives in ~/tzpro-personal/)
# ──────────────────────────────────────────────
vessel_state.jsonl
*.db
*.sqlite
*.sqlite3
captures/
voice/
logs/
lora/
sessions/
exports/
daily_log/
vault.dpapi
vault.*
*.local.json

# ──────────────────────────────────────────────
# Runtime artifacts
# ──────────────────────────────────────────────
*.pid
*.lock
*.stamp.json
.last_*
.heartbeat
.alert_state.json
.consensus_state.json
.consensus_upgrade_state.json
.tide_pool.json
.holdfast.json
.holdfast_queue.json
.tool_output.json
.vocabulary_cache.json

# ──────────────────────────────────────────────
# Python / build artifacts
# ──────────────────────────────────────────────
__pycache__/
*.pyc
*.pyo
.venv/
.pytest_cache/
.mypy_cache/
.ruff_cache/

# ──────────────────────────────────────────────
# IDE / OS
# ──────────────────────────────────────────────
.vscode/
.idea/
.DS_Store
Thumbs.db

# ──────────────────────────────────────────────
# Cloudflare / tunnel state
# ──────────────────────────────────────────────
.cloudflared/
.wrangler/

# ──────────────────────────────────────────────
# Cascade pipeline output (heavy, regenerable)
# ──────────────────────────────────────────────
cascade_out/
```

When in doubt, **add it to .gitignore**. The cost of ignoring a
non-personal file is one git-add `-f`. The cost of committing a
personal file is removing it from history.

---

## The path configuration layer

The suit has a single, sanctioned way to find the personal folder:

```python
# vessel_config.py (in repo, generic)
import os
from pathlib import Path

def personal_dir() -> Path:
    """Return the personal folder, with these precedence rules:
    1. explicit override via env var TZPRO_PERSONAL_DIR
    2. sibling folder named tzpro-personal next to this repo
    3. fallback to ./personal/ for portable demos (with warning)
    """
    explicit = os.environ.get("TZPRO_PERSONAL_DIR")
    if explicit:
        return Path(explicit).expanduser().resolve()
    
    # Sibling convention
    sibling = Path(__file__).resolve().parent.parent / "tzpro-personal"
    if sibling.exists():
        return sibling
    
    # Fallback for demo mode
    fallback = Path(__file__).resolve().parent / "personal"
    if not fallback.exists():
        print(f"WARNING: no personal folder found; using {fallback} "
              f"(create {'~'+str(sibling)} or set TZPRO_PERSONAL_DIR)")
    return fallback
```

Every module that needs to read or write personal data calls
`personal_dir()` and joins against the result. **Never hard-code
a path into the suit.**

Examples:

```python
# capture_daemon.py — where to write the NMEA stream
PERSONAL = personal_dir()
VESSEL_STATE = PERSONAL / "vessel_state.jsonl"

# dashboard.py — where to read captures from
PERSONAL = personal_dir()
CAPTURES_DIR = PERSONAL / "captures"

# vault.py — where the vault file lives
PERSONAL = personal_dir()
VAULT_FILE = PERSONAL / "vault.dpapi"
```

The captain can:

- Leave the default (sibling folder convention)
- Override via `TZPRO_PERSONAL_DIR` env var
- Symlink the sibling folder to anywhere (e.g., a NAS, an external SSD)

---

## The vessel_config.local.json schema

The personal folder has a single config file that overrides the
suit's defaults. Its schema (committed to the suit):

```json
// vessel_config.local.json — the personal schema
{
    // Vessel identity (private, never committed)
    "vessel_name": "F/V Eileen",
    "vessel_mmsi": 366999999,
    "vessel_hull_id": null,
    "vessel_home_port": "Gloucester, MA",
    
    // Hardware paths (private, hardware-specific)
    "nmea_port": "COM6",
    "thermal_camera_index": 1,
    "thermal_camera_resolution": [320, 240],
    "thermal_camera_cadence_s": 60,
    "underwater_camera_index": 0,
    "underwater_camera_resolution": [1920, 1080],
    "underwater_camera_cadence_s": 300,
    "audio_device_index": 2,
    "audio_capture_during_phases": ["trolling", "slow_cruise"],
    
    // Engine configuration (private, model-specific)
    "engine": {
        "make": "Detroit Diesel",
        "model": "6-71",
        "serial": null,
        "transmission": "Twin Disc 509",
        "rated_rpm": 1800,
        "operating_rpm_idle": 600,
        "operating_rpm_trolling": 1100,
        "operating_rpm_cruise": 1600,
        "temp_normal_f": 175,
        "temp_alarm_f": 200,
        "oil_pressure_normal_psi": 50,
        "oil_pressure_alarm_low_psi": 25,
        "coolant_temp_normal_f": 170,
        "coolant_temp_alarm_f": 195
    },
    
    // Operational thresholds (private, fishery-specific)
    "thresholds": {
        "depth_fathoms_min": 5,
        "depth_fathoms_max": 80,
        "target_species": ["atlantic_cod", "haddock", "pollock"],
        "thermocline_min_delta_f": 4,
        "speed_knots_for_fishing": 1.5
    },
    
    // Network (private, boat-specific)
    "network": {
        "lan_dashboard_port": 8090,
        "vessel_ssid": "Eileen-5GHz",
        "static_ip": "192.168.1.10"
    },
    
    // Cloud (private, quota-specific)
    "cloud": {
        "default_provider": "anthropic",
        "monthly_budget_usd": 5.00,
        "free_tier_only": true,
        "throttle_strategy": "off_peak_only"
    },
    
    // Privacy (private, override-only)
    "privacy": {
        "export_anonymize_position": false,
        "export_anonymize_vessel_name": true,
        "export_redact_voice": true
    }
}
```

The suit reads this on startup. If absent, it loads the defaults
from `vessel_config.py` (in the repo, generic). The two merge in
Python: defaults + local overrides.

The suit writes **never** to `vessel_config.local.json`. The file
is human-owned; the suit reads it. The captain changes thresholds
in their editor; the suit learns them on next launch.

---

## The vault (private credentials)

The vault lives in `vault.dpapi` in the personal folder. The vault
file is DPAPI-encrypted by `vault.py` (in the suit).

```python
# vault.py — interface (in repo)
class Vault:
    def __init__(self, path: Path = None):
        self.path = path or personal_dir() / "vault.dpapi"
    
    def get(self, key: str) -> str:
        """Decrypt and return the value for `key`, or None if not set."""
        ...
    
    def set(self, key: str, value: str) -> None:
        """Encrypt and persist `key` = `value`."""
        ...
    
    def list(self) -> list[str]:
        """Return the names of all stored keys (values stay encrypted)."""
        ...
```

The captain's vault contains:

```json
{
    "encrypted_data": "DPAPI-encrypted blob",
    "metadata": {
        "version": 1,
        "created_at": "2026-07-23T...",
        "description": "F/V Eileen credentials"
    }
}
```

The vault file is unreadable without the Windows user account that
created it. Other users on the same machine see the file but cannot
decrypt it. Backup tools that copy the file get an unreadable blob.

The vault is **never** committed. The vault mechanism (`vault.py`)
**is** committed. The vault contents (**personal**) are not.

---

## The schema repository (public, in the repo)

The schema lives in the suit. It defines the shape of Moments,
Captures, Alerts, etc. without the data.

```python
# schema/moment.py — in repo, public
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Optional

@dataclass
class Moment:
    """The atomic unit of capture. Every sensor emits into this shape."""
    timestamp: datetime
    lat: float
    lon: float
    source: str  # "nmea", "thermal", "underwater", "audio", "voice", "analysis"
    payload_ref: Optional[Path] = None  # path to binary if present
    payload_inline: Optional[dict] = None  # small JSON if no binary
    tags: list[str] = field(default_factory=list)
    confidence: float = 1.0
    schema_version: int = 1
```

The schema can be imported by the suit and by the personal code.
The data that fills the schema (`Moment(timestamp=..., lat=..., ...)`)
lives in the personal folder.

---

## The data flow (where data goes)

```
┌─────────────────┐
│ SENSORS         │  NMEA, thermal, underwater, audio, voice
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ CAPTURE (suit)  │  capture_daemon.py, nmea_bridge.py
└────────┬────────┘
         │ writes to ~/tzpro-personal/...
         ▼
┌─────────────────────────────────────────────┐
│ PERSONAL STORAGE (~/tzpro-personal/)        │
│  ├─ vessel_state.jsonl (append-only log)    │
│  ├─ captures/ (binary data)                 │
│  ├─ vault.dpapi (encrypted credentials)     │
│  ├─ vessel_config.local.json (overrides)    │
│  └─ vessel.db (SQLite index)                │
└────────┬────────────────────────────────────┘
         │ reads from
         ▼
┌─────────────────┐
│ ANALYZE (suit)  │  analyzer.py, cascade/, providers/
└────────┬────────┘
         │ emits into
         ▼
┌─────────────────────────────────────────────┐
│ PERSONAL STORAGE (continued)                │
│  └─ vessel.db (analyses, embeddings, tags)  │
└────────┬────────────────────────────────────┘
         │ reads from
         ▼
┌─────────────────┐
│ SERVE (suit)    │  dashboard.py, tray_app.py
└────────┬────────┘
         │ via HTTP on 192.168.1.10:8090
         ▼
┌─────────────────┐
│ DEVICES (LAN)   │  tablet, phone, laptop
└─────────────────┘
```

The suit never holds data. The suit is a *processor* and a *server*.
The personal folder is the *storage*. The captain's devices are
*clients*. The boundary is sharp.

---

## The "what about logs and temp files" rule

The suit writes some things during operation:

- **Logs** (current.log, capture.log): committed? NO. Lives in `~/tzpro-personal/logs/`.
- **Stampfiles** (capture_daemon.stamp.json): committed? NO. Lives in `~/tzpro-personal/`.
- **Heartbeat files** (.last_nmea_heartbeat): committed? NO. Lives in `~/tzpro-personal/`.
- **Doctor scratch files** (doctor.tmp.json): committed? NO. Lives in `~/tzpro-personal/`.
- **Cache files** (*.cache.json): committed? NO. Lives in `~/tzpro-personal/`.

The rule: **if the suit generates it at runtime, it lives in the
personal folder**. The repo contains only code, schemas, tests,
docs, and fixtures.

For tests, the suite writes to `tests/tmp/` which is in `.gitignore`
and auto-cleaned by pytest.

---

## The "what about BATON_PASS and session logs" rule

`docs/BATON_PASS_<date>.md` in the repo is a **template**. The
content is synthesized or sanitized to remove personal operation
details. The structure is preserved.

The captain's actual session handoffs, with their real data, real
discoveries, real decisions, live in `~/tzpro-personal/sessions/`.

The repo can have multiple `BATON_PASS_<date>.md` files showing
the format. None of them contain real data.

```markdown
# BATON_PASS_TEMPLATE.md (in repo)

## Summary
[Summarize what was being worked on — generic, not operation-specific]

## What changed
- [List of files changed in the repo]
- [List of features added — generic descriptions]

## What's next
- [Next bullet from the daily plan]

## Open questions
- [Issues that need human input]

## Context budget
- [What tier was the agent in]
```

The template is committed. The filled-in version is personal.

---

## The "what about contributions back to the public repo" rule

When you (the captain or a contributor) want to upstream a change
from your private fork to the public suit:

1. **Review the diff** for any personal data leaking through:
   - API keys, tokens.
   - Real coordinates, real vessel names.
   - Personal notes or observations.
   - Paths into your personal folder.
2. **If the contribution is generic** (a new schema field, a new
   capture source, a new analyzer): open a PR. The suit grows.
3. **If the contribution is personalized** (your specific
   thresholds, your specific LoRA, your specific chart plots):
   - Either genericize it (replace personal values with config keys)
   - Or keep it in your private fork.

The PR review process is the last line of defense. Reviewers
should reject any PR that introduces personal data.

---

## The "what about cloud sync" question (Phase 8)

Cloud sync (Phase 8) will use Cloudflare Workers, R2, D1, Vectorize.

The privacy boundary gets explicit:

- **Vectorize** stores embeddings only, not raw data. Embeddings
  can leak some signal — they should be of *anonymized* Moments.
- **D1** stores the schema-validated metadata: timestamp, lat/lon
  (optionally fuzzy), source, tags. Not raw payloads.
- **R2** stores captured images. These are the most sensitive.
  Cloud sync should be **opt-in per Moment class**, with the
  captain able to mark certain captures as "local-only".

The captain's `vessel_config.local.json` has a `cloud` section:

```json
"cloud": {
    "sync_enabled": false,
    "sync_anonymize_position": true,
    "sync_redact_voice": true,
    "sync_redact_engine_serial": true,
    "whitelist_capture_sources": ["sounder", "echogram"],
    "blacklist_capture_sources": ["thermal", "voice", "underwater"]
}
```

By default, cloud sync is **disabled**. The captain opts in per
source. Theuit never enables cloud sync without an explicit
config setting.

---

## The "what about the agent's own session logs" question

When the agent runs in a session, it produces diagnostic output
(stdout, logs, doctor reports). Where does that go?

- **stdout** (terminal output during interactive use): the
  captain's terminal. Not committed. Not persisted unless the
  captain redirects it.
- **lesson_plan entries** (today's plan, weekly plan): the
  suit's `docs/PLANS/` directory. **Committed as templates.**
  Personal session notes with real data go in `~/tzpro-personal/sessions/`.
- **doctor reports** (doctor.json, doctor.jsonl): local to the
  personal folder. Never committed.
- **delegation task outputs** (delegation/tasks/*.response.json):
  **committed** if the task is generic (a code review, a test
  scaffold). **Not committed** if the task generated personal
  insights or analysis.

The principle: **the suit's templates are public. The agent's
applied work on real data is private.**

---

## The "how does the agent know what is personal" implementation

The agent should check, before every write:

1. **Is the path inside the repo?** If yes, the content must be
   generic (template, schema, code, fixture, doc).
2. **Is the path inside the personal folder?** If yes, anything
   goes — it's the person's storage.
3. **Is the path somewhere else?** The agent should log a warning
   and ask the captain. Write paths should be in the personal
   folder, not in random locations.

The `personal_dir()` function is the only sanctioned path. The
agent does not invent new paths inside the repo.

---

## The "what about the next agent's bootstrap" principle

When a new agent (or a new captain) opens the repo for the first
time, they should:

1. Read `docs/SUIT_VS_PERSON.md` — the charter.
2. Read `docs/architecture/PRIVACY_BOUNDARY.md` (this file) — the spec.
3. Run `python setup.py check-privacy` — confirms the personal
   folder exists and the repo is clean.
4. Read `docs/AGENT_OPERATING_MODEL.md` — the doctrine.
5. Then proceed with the normal bootstrap.

This is the **first** thing the agent does. If the agent doesn't
understand the suit/person boundary, it cannot operate safely.

---

## See also

- `docs/SUIT_VS_PERSON.md` — the charter
- `docs/AGENT_OPERATING_MODEL.md` — the doctrine
- `docs/BOOTCAMP.md` — the operating rules (add the privacy rule)
- `docs/architecture/CAPTURE_PIPELINE.md` — the multi-modal capture pipeline (the senses)
- `docs/ROADMAP.md` — phase index (Phase-∞ is the viral mechanic)
- `docs/README.md` — repo entry point (will be added)
