# Dashboard — `dashboard.py`

**File:** `dashboard.py` (~600 lines)
**Priority:** UX — captain-facing LAN web UI.
**Owner:** tzpro-agent.

## What It Does

A **Starlette** ASGI app that serves a single-page web UI over the
local LAN on port `:8090`. Lets the captain (or any device on the
boat's wifi) see:

- Live vessel state (lat/lon/SOG/COG/state_class from the bridge)
- Today's capture list with thumbnails
- A chat panel for asking questions of the AI agent
- Provider status & switching (enable/disable ollama, deepinfra, etc.)
- Embedding controls

## Stack

- **Starlette** ASGI (not FastAPI — keeps deps minimal)
- **uvicorn** server
- **Jinja2** templates for the HTML shell
- Plain JS for the SPA-ish client behavior

## Routes (`dashboard.py`, search `Route(`)

| Route | Method | Handler | Purpose |
|---|---|---|---|
| `/` | GET | `index` | SPA shell (Jinja template) |
| `/api/health` | GET | `api_health` | Liveness |
| `/api/vessel` | GET | `api_vessel` | Latest vessel state (proxied from bridge :8654) |
| `/api/providers` | GET | `api_providers` | List provider states |
| `/api/providers/{name}` | POST | `api_provider_set` | Enable / disable a provider |
| `/api/captures` | GET | `api_captures` | List captures (today by default, paginable) |
| `/api/captures/{capture_id}` | GET | `api_capture_one` | One capture's metadata (JSON twin) |
| `/api/captures/{capture_id}/file` | GET | `api_capture_file` | Serve the PNG inline |
| `/api/chat` | POST | `api_chat` | Send a message to the agent |
| `/api/embed` | POST | `api_embed` | Embed a text string |

## Bridge Proxy

`api_vessel` proxies to `http://127.0.0.1:8654/vessel` so the dashboard
doesn't have to parse NMEA directly. If the bridge is down, the route
returns a 503 with `{"error": "bridge_down", "hint": "see fix_priority0.bat"}`.

## Captures Browser

`_list_captures()` (≈`dashboard.py:60`) walks `captures/v3/*/` and
returns a list of dicts:

```json
[
  {
    "capture_id": "0840_5547.643N_13140.201W",
    "day_folder": "2026-07-23_5547N_13141W",
    "ts_local": "2026-07-23T08:40:00-08:00",
    "ts_local_hhmm": "0840",
    "lat": 55.79405, "lon": -131.67002,
    "has_png": true, "has_json": true, "has_md": true,
    "thumb_url": "/api/captures/0840_5547.643N_13140.201W/file"
  },
  …
]
```

The HTML template renders a grid of thumbs; clicking opens the
captures' `.md` annotation.

## Chat Endpoint

`api_chat` (`dashboard.py`, near bottom of routes) takes:

```json
{
  "message": "What was the catch rate yesterday?",
  "provider": "ollama",
  "model": "granite4.1:8b"
}
```

… and returns a streaming-style response from `agent_router.py`.
The dashboard's chat panel is the human-facing front to the entire
AI stack — `agent_router` → `providers/*` → response.

## Run

```bash
python dashboard.py
```

Or via `fix_priority0.bat` which also (re)starts the bridge.

Listens on `0.0.0.0:8090` so it's reachable from phone/tablet on
the same wifi as the laptop.

## Verified Line Numbers (as of `b64c2ef`)

- `_list_captures`: ~`dashboard.py:60`
- Route definitions: search `dashboard.py` for `Route("`
- `api_chat`: search `dashboard.py` for `def api_chat`

## Configuration

Most behavior is driven by `vessel.json` (provider chain, task routing).
Edit there, then restart dashboard:

```bash
powershell -Command "Get-Process python | Where-Object { $_.CommandLine -like '*dashboard*' } | Stop-Process -Force"
python dashboard.py
```

## Common Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| 503 from `/api/vessel` | Bridge down | `fix_priority0.bat` |
| Dashboard won't start | Port 8090 in use | Stop the other listener, retry |
| `/api/captures` empty | `captures/v3/` empty or missing | Verify capture_daemon is running |
| Chat fails with "provider unavailable" | Provider disabled or model not pulled | Check `/api/providers`; `ollama pull granite4.1:8b` |

## Related Docs

- `docs/engineer/01_nmea_bridge.md` — vessel data source
- `docs/engineer/03_capture_v3.md` — capture data source
- `docs/engineer/08_providers.md` — LLM providers the dashboard uses
- `docs/architecture/PROJECTION_LAYER.md` — what the dashboard is *part of*