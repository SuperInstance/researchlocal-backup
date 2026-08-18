"""agent_router.py — picks a provider+model for a given task and runs it.

Combines:
  - vessel.json (which providers are enabled + their config + routes)
  - vault      (the api keys, decrypted on demand)
  - providers/ (the actual ModelProvider implementations)

Usage:
    from agent_router import AgentRouter, Task
    r = AgentRouter()
    resp = r.chat(Task.CHAT_QUICK, [ChatMessage("user", "hi")])
    print(resp.content)
"""
from __future__ import annotations

import os
from dataclasses import dataclass
from enum import Enum
from typing import Optional

import vessel_config
from providers import (
    ModelProvider, ChatMessage, ChatRequest, ChatResponse,
    EmbedRequest, EmbedResponse, get_provider,
)


class Task(str, Enum):
    PERCEPTION_CHECK_1MIN = "perception_check_1min"
    SCREENSHOT_10MIN      = "screenshot_10min"
    HOURLY_REVIEW         = "hourly_review"
    EVENING_DEBRIEF       = "evening_debrief"
    MIDDAY_REPORT         = "midday_report"
    CHAT_QUICK            = "chat_quick"
    CHAT_HEAVY            = "chat_heavy"
    VISION_ANALYZE        = "vision_analyze"
    EMBEDDING             = "embedding"


@dataclass
class AgentRouter:
    """Routes a task to the first available provider/model in the
    route list. If every route fails, returns the last error as the
    response content (so the UI can still show something).
    """
    config_path: Optional[str] = None

    def _cfg(self, force: bool = False) -> dict:
        if self.config_path:
            return vessel_config.load(Path(self.config_path), force_reload=force)
        return vessel_config.load(force_reload=force)

    def _provider_with_key(self, pname: str, prov_cfg: dict) -> Optional[ModelProvider]:
        """Build a provider, merging api_key from the vault if the
        provider's config references a vault entry by name."""
        cfg = dict(prov_cfg.get("config", {}))
        # If the provider config has 'vault_key', pull the api_key
        # (and optionally base_url) from the encrypted vault.
        vault_key_name = cfg.pop("vault_key", None)
        if vault_key_name:
            try:
                from vault import Vault  # type: ignore
                v = Vault()
                secret = v.try_get(vault_key_name)
                if secret:
                    if "api_key" in secret and not cfg.get("api_key"):
                        cfg["api_key"] = secret["api_key"]
                    if "base_url" in secret and not cfg.get("base_url"):
                        cfg["base_url"] = secret["base_url"]
            except Exception:
                # Vault unavailable or key missing — let the provider
                # report itself unavailable, which the router will skip.
                pass
        try:
            return get_provider(pname, cfg)
        except Exception:
            return None

    def _route_for(self, task: Task, cfg: dict) -> list[tuple[str, dict]]:
        """Returns [(provider_name, merged_provider_config)] for the
        task, skipping providers that are not enabled. Each entry
        carries the per-route overrides (model, temperature, max_tokens)."""
        routes = cfg.get("task_routing", {}).get(task.value, [])
        provs = cfg.get("providers", {})
        out = []
        for r in routes:
            pname = r.get("provider")
            if not pname:
                continue
            pe = provs.get(pname, {})
            if not pe.get("enabled", False):
                continue
            merged = {
                "enabled": True,
                "config": dict(pe.get("config", {})),
                "models": pe.get("models", []),
                "_route": r,   # route-level overrides
            }
            out.append((pname, merged))
        return out

    def chat(self, task: Task,
             messages: list[ChatMessage],
             temperature: float = 0.7,
             max_tokens: int = 1024) -> ChatResponse:
        cfg = self._cfg()
        for pname, merged in self._route_for(task, cfg):
            prov = self._provider_with_key(pname, merged)
            if prov is None or not prov.is_available():
                continue
            route = merged.get("_route", {})
            req = ChatRequest(
                messages=messages,
                model=route.get("model", ""),
                temperature=route.get("temperature", temperature),
                max_tokens=route.get("max_tokens", max_tokens),
                task=task.value,
            )
            try:
                resp = prov.chat(req)
                if resp.content and not resp.content.startswith("("):
                    return resp
                # Provider returned a synthetic error string — try next.
                last_err = resp
            except Exception as e:
                last_err = ChatResponse(
                    content=f"({pname} threw {type(e).__name__}: {e})",
                    provider=pname,
                )
                continue

        # All routes failed.
        if 'last_err' in locals():
            return last_err  # type: ignore[return-value]
        return ChatResponse(
            content=f"(router: no route for task={task.value} or no enabled providers)",
            provider="router",
        )

    def embed(self, texts: list[str], model: str = "") -> EmbedResponse:
        cfg = self._cfg()
        for pname, merged in self._route_for(Task.EMBEDDING, cfg):
            prov = self._provider_with_key(pname, merged)
            if prov is None or not prov.is_available():
                continue
            req = EmbedRequest(texts=texts, model=model)
            try:
                return prov.embed(req)
            except Exception:
                continue
        return EmbedResponse(vectors=[[]], provider="router", dim=0)