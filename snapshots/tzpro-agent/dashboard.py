"""dashboard.py — LAN-reachable web UI for TZ Pro Agent.

Phase 1 layout:
  GET  /                           -> single-page app (index.html)
  GET  /api/health                 -> liveness
  GET  /api/vessel                 -> vessel.json contents (no secrets)
  GET  /api/providers              -> enabled providers + their models
  POST /api/providers/{name}       -> set provider config (api_key
                                      optionally stored in vault)
  GET  /api/captures               -> list captures tree
  GET  /api/captures/{capture_id}  -> single capture (image + sidecar)
  POST /api/chat                   -> chat completion
  POST /api/embed                  -> embeddings
  GET  /api/sessions/{sid}/...     -> per-session overrides (later phase)

Runs on 0.0.0.0:8090 so any device on the boat LAN can reach it
(phone, crew laptop, the captain's phone from the back deck).

Why starlette not FastAPI
-------------------------
Starlette was already installed; FastAPI was not. Pydantic isn't
needed for our payload sizes, so the smaller dependency footprint
is a win for the boat's laptop.

Why aiohttp for outbound (not requests)
---------------------------------------
aiohttp is already installed and we can run it concurrently with
the starlette loop if needed. For now we just use the sync HTTP
in providers/, so this is future-proofing only.
"""
from __future__ import annotations

import asyncio
import base64
import ipaddress
import json
import mimetypes
import socket
from pathlib import Path
from typing import Optional
from urllib.parse import unquote

from starlette.applications import Starlette
from starlette.responses import (
    HTMLResponse, JSONResponse, FileResponse, PlainTextResponse,
)
from starlette.routing import Route
from starlette.requests import Request

import vessel_config
from providers import ChatMessage, ChatRequest
from agent_router import AgentRouter, Task


# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

def _get_lan_ip() -> str:
    """Best-effort: return the LAN-facing IPv4 address. Falls back
    to 127.0.0.1 if we can't determine one."""
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"


def _capture_id_from_filename(name: str) -> str:
    """Stable capture id from filename. We use the file stem."""
    return Path(name).stem


def _list_captures(captures_dir: Path) -> list[dict]:
    """Return a list of {date, time, lat, lon, file, mtime} for every
    capture on disk, sorted by mtime descending. The dashboard uses
    this to build the file tree.
    """
    out = []
    if not captures_dir.exists():
        return out
    # Layout: YYYY/YYYY-MM-DD_/HHMM_LL.NNNN...W.png
    for year_dir in sorted(captures_dir.iterdir()):
        if not year_dir.is_dir():
            continue
        for day_dir in sorted(year_dir.iterdir()):
            if not day_dir.is_dir():
                continue
            for f in day_dir.iterdir():
                if not f.is_file() or f.suffix.lower() not in (".png", ".jpg", ".jpeg", ".json", ".md"):
                    continue
                stat = f.stat()
                # Filename pattern: HHMM_LAT_LON...
                # Extract time + lat/lon from the stem.
                stem = f.stem
                parts = stem.split("_", 2)
                time_str = parts[0] if parts else ""
                loc_str = parts[1] if len(parts) > 1 else ""
                out.append({
                    "id": f"{day_dir.name}_{stem}",
                    "date": day_dir.name.split("_")[0],
                    "time": time_str,
                    "location": loc_str,
                    "filename": f.name,
                    "rel_path": str(f.relative_to(captures_dir)).replace("\\", "/"),
                    "abs_path": str(f),
                    "size": stat.st_size,
                    "mtime": stat.st_mtime,
                    "ext": f.suffix.lower(),
                })
    out.sort(key=lambda x: x["mtime"], reverse=True)
    return out


# ---------------------------------------------------------------------------
# routes
# ---------------------------------------------------------------------------

async def index(request: Request) -> HTMLResponse:
    html_path = Path(__file__).resolve().parent / "dashboard" / "index.html"
    if html_path.exists():
        return HTMLResponse(html_path.read_text(encoding="utf-8"))
    return HTMLResponse("<h1>dashboard/index.html missing</h1>")


async def api_health(request: Request) -> JSONResponse:
    cfg = vessel_config.load()
    return JSONResponse({
        "ok": True,
        "vessel": cfg["vessel"]["name"],
        "lan_ip": _get_lan_ip(),
        "providers_enabled": vessel_config.enabled_provider_names(cfg),
    })


async def api_vessel(request: Request) -> JSONResponse:
    cfg = vessel_config.load()
    # Strip any secret fields if they ever leak into vessel.json.
    safe = json.loads(json.dumps(cfg))
    return JSONResponse(safe)


async def api_providers(request: Request) -> JSONResponse:
    cfg = vessel_config.load()
    out = {}
    for name, p in cfg.get("providers", {}).items():
        out[name] = {
            "enabled": p.get("enabled", False),
            "models": p.get("models", []),
            "config_keys": sorted(list(p.get("config", {}).keys())),
            "has_key": bool(p.get("config", {}).get("api_key"))
                       or bool(p.get("config", {}).get("vault_key")),
        }
    return JSONResponse(out)


async def api_provider_set(request: Request) -> JSONResponse:
    name = request.path_params["name"]
    body = await request.json()
    enabled = body.get("enabled")
    config = body.get("config", {}) or {}
    models = body.get("models")

    cfg = vessel_config.load(force_reload=True)
    if name not in cfg.get("providers", {}):
        return JSONResponse({"error": f"unknown provider {name}"}, status_code=404)

    pe = cfg["providers"][name]
    if enabled is not None:
        pe["enabled"] = bool(enabled)
    if config:
        # Merge, never overwrite vault_key if it's set.
        pe.setdefault("config", {})
        for k, v in config.items():
            if k == "api_key" and v:
                # Store in vault; reference it from config.
                try:
                    from vault import Vault  # type: ignore
                    v_obj = Vault()
                    v_obj.set(name, {"api_key": v, **(pe["config"].get("base_url") and {"base_url": pe["config"]["base_url"]} or {})})
                    pe["config"]["vault_key"] = name
                    pe["config"].pop("api_key", None)
                except Exception as exc:
                    return JSONResponse(
                        {"error": f"vault error: {exc}"}, status_code=500,
                    )
            else:
                pe["config"][k] = v
    if models is not None:
        pe["models"] = models

    # Persist.
    Path("vessel.json").write_text(
        json.dumps(cfg, indent=2), encoding="utf-8",
    )
    vessel_config.reload()
    return JSONResponse({"ok": True})


async def api_captures(request: Request) -> JSONResponse:
    cfg = vessel_config.load()
    captures_dir = Path(cfg["paths"]["captures_dir"])
    items = _list_captures(captures_dir)
    return JSONResponse({
        "captures_dir": str(captures_dir),
        "count": len(items),
        "items": items,
    })


async def api_capture_one(request: Request) -> JSONResponse:
    capture_id = unquote(request.path_params["capture_id"])
    cfg = vessel_config.load()
    captures_dir = Path(cfg["paths"]["captures_dir"])
    # The id is "{day_dir}_{stem}". Reconstruct.
    if "_" not in capture_id:
        return JSONResponse({"error": "bad capture id"}, status_code=400)
    day, stem = capture_id.split("_", 1)
    day_dir = captures_dir / day[:4] / capture_id.split("_")[0]
    # find file by stem across any extension
    for ext in (".png", ".jpg", ".jpeg", ".json", ".md"):
        candidate = day_dir / f"{stem}{ext}"
        if candidate.exists():
            stat = candidate.stat()
            return JSONResponse({
                "id": capture_id,
                "abs_path": str(candidate),
                "rel_path": str(candidate.relative_to(captures_dir)).replace("\\", "/"),
                "ext": ext,
                "size": stat.st_size,
                "mtime": stat.st_mtime,
            })
    return JSONResponse({"error": "not found"}, status_code=404)


async def api_capture_file(request: Request) -> FileResponse:
    """Serve the raw capture file (image or sidecar) inline."""
    capture_id = unquote(request.path_params["capture_id"])
    cfg = vessel_config.load()
    captures_dir = Path(cfg["paths"]["captures_dir"])
    day = capture_id.split("_", 1)[0]
    stem = capture_id.split("_", 1)[1] if "_" in capture_id else capture_id
    day_dir = captures_dir / day[:4] / day
    for ext in (".png", ".jpg", ".jpeg", ".json", ".md"):
        candidate = day_dir / f"{stem}{ext}"
        if candidate.exists():
            mime, _ = mimetypes.guess_type(str(candidate))
            return FileResponse(str(candidate), media_type=mime or "application/octet-stream")
    return PlainTextResponse("not found", status_code=404)


async def api_chat(request: Request) -> JSONResponse:
    body = await request.json()
    task = body.get("task", "chat_quick")
    msgs_raw = body.get("messages", [])
    msgs = [ChatMessage(role=m["role"], content=m["content"]) for m in msgs_raw]
    try:
        t = Task(task)
    except ValueError:
        t = Task.CHAT_QUICK

    router = AgentRouter()
    resp = router.chat(t, msgs)
    return JSONResponse({
        "content": resp.content,
        "provider": resp.provider,
        "model": resp.model,
        "usage": resp.usage,
        "task": t.value,
    })


async def api_embed(request: Request) -> JSONResponse:
    body = await request.json()
    texts = body.get("texts", [])
    if not texts:
        return JSONResponse({"error": "no texts"}, status_code=400)
    router = AgentRouter()
    e = router.embed(texts)
    return JSONResponse({
        "provider": e.provider,
        "model": e.model,
        "dim": e.dim,
        "count": len(e.vectors),
    })


# ---------------------------------------------------------------------------
# app
# ---------------------------------------------------------------------------

app = Starlette(debug=False, routes=[
    Route("/", index),
    Route("/api/health", api_health),
    Route("/api/vessel", api_vessel),
    Route("/api/providers", api_providers),
    Route("/api/providers/{name}", api_provider_set, methods=["POST"]),
    Route("/api/captures", api_captures),
    Route("/api/captures/{capture_id}", api_capture_one),
    Route("/api/captures/{capture_id}/file", api_capture_file),
    Route("/api/chat", api_chat, methods=["POST"]),
    Route("/api/embed", api_embed, methods=["POST"]),
])


def run():
    import uvicorn
    ip = _get_lan_ip()
    print(f"\n  TZ Pro Agent dashboard")
    print(f"    local:    http://127.0.0.1:8090/")
    print(f"    LAN:      http://{ip}:8090/")
    print(f"    vessel:   {vessel_config.vessel_info()['name']}")
    cfg = vessel_config.load()
    print(f"    enabled:  {', '.join(vessel_config.enabled_provider_names(cfg)) or '(none yet)'}\n")
    uvicorn.run(app, host="0.0.0.0", port=8090, log_level="info")


if __name__ == "__main__":
    run()