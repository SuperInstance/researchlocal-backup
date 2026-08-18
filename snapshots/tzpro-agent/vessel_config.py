"""vessel_config.py — load and query vessel.json.

This is the single read interface to vessel.json. Anything that needs
vessel info, provider config, or task routing should call this module
rather than re-parsing the JSON.

The file is intentionally editable by humans AND by the agent itself,
so we cache it in-memory but reload on demand with reload().
"""
from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Optional


DEFAULT_PATH = Path(__file__).resolve().parent / "vessel.json"


_cache: Optional[dict] = None
_cache_path: Optional[Path] = None


def _load(path: Path) -> dict:
    if not path.exists():
        raise FileNotFoundError(
            f"vessel.json not found at {path}. "
            f"Run from the tzpro-agent repo root or set TZPRO_VESSEL_CONFIG."
        )
    return json.loads(path.read_text(encoding="utf-8"))


def load(path: Optional[Path] = None, *, force_reload: bool = False) -> dict:
    """Load (and cache) the vessel config. Pass force_reload=True to
    re-read from disk (used after the agent edits vessel.json)."""
    global _cache, _cache_path
    p = Path(path) if path else Path(os.environ.get("TZPRO_VESSEL_CONFIG", DEFAULT_PATH))
    if force_reload or _cache is None or _cache_path != p:
        _cache = _load(p)
        _cache_path = p
    return _cache


def reload() -> dict:
    """Force-reload vessel.json from disk."""
    return load(force_reload=True)


def vessel_info(cfg: Optional[dict] = None) -> dict:
    return (cfg or load())["vessel"]


def paths(cfg: Optional[dict] = None) -> dict:
    return (cfg or load()).get("paths", {})


def providers(cfg: Optional[dict] = None) -> dict:
    return (cfg or load()).get("providers", {})


def provider_config(name: str, cfg: Optional[dict] = None) -> dict:
    """Return {enabled, config, models} for one provider."""
    p = providers(cfg).get(name, {})
    return {
        "enabled": p.get("enabled", False),
        "config": p.get("config", {}),
        "models": p.get("models", []),
    }


def task_route(task: str, cfg: Optional[dict] = None) -> list[dict]:
    """Return the ordered route list for a named task.
    Empty list if the task is unknown (caller should fall back)."""
    return (cfg or load()).get("task_routing", {}).get(task, [])


def enabled_provider_names(cfg: Optional[dict] = None) -> list[str]:
    return [n for n, p in providers(cfg).items() if p.get("enabled")]