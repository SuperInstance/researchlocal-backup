# Phase 1 — Capture + Tray + LAN Dashboard + Provider Onboarding

> **Status:** CURRENT
> **Goal:** Local app working perfectly on the ProArt, reachable from any
> LAN device, with secure provider keys.

## What ships in Phase 1

1. **Capture daemon** that follows TZ Pro's lifecycle — starts when
   `TimeZero.exe` appears, stops cleanly when it exits
2. **Tray app** with right-click menu: Start/Stop, Open Dashboard, Status,
   Open Captures Folder, Quit
3. **LAN-reachable dashboard** on `0.0.0.0:8090` with:
   - 3-panel layout (files / capture+analysis / chat)
   - File tree organized by date with distance filter
   - Date-range search bar
   - Markdown / Code / Raw JSON view switcher for analysis
   - Per-session chat with provider onboarding form
4. **Provider onboarding** in the right panel:
   - API base URL field
   - API key field (stored in Windows DPAPI vault)
   - Provider dropdown (DeepInfra, OpenAI, OpenRouter, Z.AI, Grok,
     DeepSeek, Custom)
   - "Get an API key here" link buttons for each provider
   - Per-task model preference (1-min, 10-min, 1-hr, evening-debrief,
     mid-day, custom)
   - Quick / Heavy model slots for chat
   - Multi-model comparison option (advanced)
5. **Secure key vault** — Windows DPAPI-encrypted, excluded from backups
6. **Backup procedure** — SQLite + JSONL + files + embeddings; vault skipped
7. **ROADMAP.md** (this file's parent) — full vision
8. **`doctor.py`** with health checks for the new components

## Architecture decisions

### Capture daemon lifecycle

- Single supervisor process (`capture_daemon.py`) polls every 5s for
  `TimeZero.exe` via `tasklist`
- When TZ Pro appears → spawns `capture_v3.py` as child process with
  `--require-tzpro` flag (which itself also checks TZ Pro so a race
  condition doesn't leave an orphan capture running)
- When TZ Pro exits → sends SIGTERM (graceful) to child, waits up to 10s,
  then SIGKILL
- Daemon writes `state.json` to disk so the tray app can show "running"
  vs "stopped" without polling the child

### Tray app

- Uses `pystray` for the system tray icon
- Icon: `assets/icon-tray.ico` (multi-resolution from your toolbar.jpg)
- Menu items:
  - Status header (disabled, shows current state)
  - Open Dashboard → `webbrowser.open("http://<lan_ip>:8090")`
  - Start/Stop Capture → toggles the supervisor
  - Open Captures Folder → `subprocess.Popen("explorer", today's folder)`
  - Quit → graceful shutdown of supervisor + exit
- Tooltip updates every 5s with: "TZPro Capture — running, last: 16:20"
  or "TZPro Capture — stopped (TZ Pro not running)"

### LAN dashboard

- FastAPI on `0.0.0.0:8090`
- Session ID via `localStorage` UUID, sent as cookie
- REST API:
  - `GET /api/sessions/{sid}/captures?distance_m=&since=&until=` → file tree
  - `GET /api/sessions/{sid}/captures/{capture_id}` → full capture + analysis
  - `POST /api/sessions/{sid}/chat` → chat message
  - `GET /api/sessions/{sid}/providers` → current provider config (no keys)
  - `POST /api/sessions/{sid}/providers` → update config (writes to vault)
  - `GET /api/health` → liveness/readiness
- Static files served from `/` (single-page HTML+JS+CSS)
- No auth — bound to LAN, captain trusts the network (documented risk)

### Provider onboarding flow

1. New user opens dashboard, right panel shows "Set up a provider to start
   chatting with your data"
2. Dropdown shows providers with "Get key →" links to:
   - [DeepInfra](https://deepinfra.com/dash/api_keys)
   - [OpenAI](https://platform.openai.com/api-keys)
   - [OpenRouter](https://openrouter.ai/keys)
   - [Z.AI](https://z.ai/manage-apikey/apikey-list)
   - [Grok (xAI)](https://console.x.ai/)
   - [DeepSeek](https://platform.deepseek.com/api_keys)
3. User pastes key, clicks "Save & Test"
4. Vault encrypts via DPAPI, makes test call (`GET /models`), shows green
   check or error
5. User then picks their per-cadence model preferences (collapsed "Advanced")

### Per-cadence model slots

```
1-min perception-check  → [skipped by default; needs local vision]
10-min screenshot       → small/cheap (deepseek-v4-flash recommended)
1-hr review             → medium (gpt-4o-mini / claude-haiku / etc.)
evening-debrief         → heavy (claude-sonnet / gpt-4o / gemini-pro)
mid-day report          → medium-heavy
custom slot             → any
```

Chat slots:
- **Quick** — fast and cheap, default deepseek-v4-flash
- **Heavy** — deeper reasoning, default claude-sonnet or gpt-4o
- **Multi-model compare** (advanced) — query N models in parallel, show
  side-by-side answers for diversity (e.g. seed-2.0-mini, qwen3.6, gemini,
  gpt-4o)

### Secure key vault

```python
# vault.py
import win32crypt  # via pywin32
class Vault:
    def __init__(self, path):  # ~/.tzpro-agent/vault.dat
        ...
    def set(self, key, value):  # encrypted at rest
        ...
    def get(self, key):  # decrypted on read
        ...
    def list(self):  # keys only, no values
        ...
```

- Encrypted blob format: JSON metadata (provider, added_at, label) +
  DPAPI-encrypted payload (api_base, api_key)
- File permissions: only current user can read
- **Backup exclusion list** is documented in `BACKUP.md` — every backup
  script MUST skip `vault.dat`
- On restore, user re-enters keys (deliberate — never restore secrets)

## Files added in Phase 1

```
tzpro-agent/
├── ROADMAP.md                    (root, points at docs/ROADMAP.md)
├── assets/
│   ├── icon-source.png
│   ├── icon-tray-16.png
│   ├── icon-tray-32.png
│   ├── icon-tray-64.png
│   ├── icon-tray.ico
│   └── icon-shortcut-256.png
├── docs/
│   ├── ROADMAP.md
│   ├── BACKUP.md                 (what to include, what to skip)
│   └── phases/
│       ├── phase-1.md  (this file)
│       ├── phase-2.md
│       └── ...
├── capture_daemon.py             (TZ Pro lifecycle supervisor)
├── capture_v3.py                 (existing, --require-tzpro added)
├── tray_app.py                   (pystray front door)
├── dashboard.py                  (FastAPI server)
├── dashboard/
│   ├── index.html                (single-page 3-panel UI)
│   ├── app.js
│   └── styles.css
├── vessel.json                   (provider config schema)
├── vault.py                      (DPAPI key storage)
├── providers/
│   ├── __init__.py
│   ├── base.py                   (ModelProvider interface)
│   ├── deepinfra.py
│   ├── openai.py
│   ├── openrouter.py
│   ├── zai.py
│   ├── grok.py
│   ├── deepseek.py
│   ├── custom.py
│   └── ollama.py
├── doctor.py                     (extended with new checks)
└── scripts/
    ├── make_shortcut.py          (creates the .lnk on Desktop)
    └── make_icon_assets.py       (regenerates icon bundle)
```

## Exit criteria

- [ ] Tray app launches on Windows startup, icon visible
- [ ] Right-click menu has all items functional
- [ ] When TZ Pro starts → capture daemon starts within 10s
- [ ] When TZ Pro exits → capture daemon stops within 10s, no orphan process
- [ ] Dashboard accessible from ProArt at `http://localhost:8090`
- [ ] Dashboard accessible from phone at `http://<proart_ip>:8090`
- [ ] File tree shows today's captures with thumbnails
- [ ] Distance filter (1/5/10mi/all) works against current GPS
- [ ] Date-range search jumps to specified range
- [ ] Markdown/Code/Raw JSON view switcher works
- [ ] Chat panel accepts message, calls provider, returns answer
- [ ] Provider onboarding saves key to vault, key not visible in any
      backup, key not visible in any log
- [ ] Per-cadence model slots saved and used
- [ ] `doctor.py` reports all components healthy

## Risks & open questions

1. **`pystray` on Windows 11** — needs verification, may need fallback
2. **LAN IP discovery** — need to pick primary NIC reliably; mDNS
   (`proart.local`) requires Bonjour or similar on the phone
3. **Multi-tab sessions** — initial impl is one session per browser;
   crew on their own phone = separate browser = separate session. ✅
4. **Windows Defender** — may quarantine `pythonw.exe` tray apps, may
   need code-signing or Defender exclusion

## What unlocks after Phase 1

- Phase 2 (analyzer wiring) is straightforward — the provider infra and
  schema are in place, just need to wire cadence → provider call →
  write `analysis` field
- The dashboard already supports per-session chat, so Phase 3
  (multi-session) is mostly UX + invite-link work
