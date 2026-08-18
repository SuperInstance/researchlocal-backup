"""providers/openai_compat.py — OpenAI-compatible chat completions.

Several providers serve an OpenAI-compatible /v1/chat/completions
endpoint, which lets us share one implementation. We give each
provider a default base URL and a 'get an api key' link for the
onboarding form.

Covered:
  - OpenAI       (https://api.openai.com/v1)
  - OpenRouter   (https://openrouter.ai/api/v1)
  - Grok         (https://api.x.ai/v1)
  - DeepSeek     (https://api.deepseek.com/v1)
  - Z.AI / Zhipu (https://api.z.ai/v1 or https://open.bigmodel.cn/api/paas/v4)

DeepInfra is OpenAI-compatible too but we give it its own subclass
because it has different defaults (free tier, vision flag) and
because it is the recommended default for Phase 1.
"""
from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Optional

from .base import (
    ModelProvider, ChatRequest, ChatResponse,
)


def _http_post_json(url: str, body: dict, headers: dict,
                     timeout_s: float) -> tuple[int, dict]:
    req = urllib.request.Request(
        url,
        data=json.dumps(body).encode("utf-8"),
        headers={"Content-Type": "application/json", **headers},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout_s) as resp:
            return resp.status, json.loads(resp.read().decode("utf-8"))
    except urllib.error.HTTPError as e:
        try:
            err_body = json.loads(e.read().decode("utf-8"))
        except Exception:
            err_body = {"error": str(e)}
        return e.code, err_body


class OpenAICompatProvider(ModelProvider):
    """Base for OpenAI-compatible APIs. Subclasses set name + defaults."""
    name = "openai_compat"
    default_base_url: str = ""
    default_model: str = ""
    key_help_url: str = ""

    def __init__(self, config: Optional[dict] = None):
        config = config or {}
        self.base_url = (
            config.get("base_url")
            or self.default_base_url
        ).rstrip("/")
        self.api_key = config.get("api_key", "")
        self.timeout_s = float(config.get("timeout_s", 60))

    def is_available(self) -> bool:
        return bool(self.api_key) and bool(self.base_url)

    def list_models(self) -> list[str]:
        # Most of these providers require a key just to list models.
        # We don't want to make a network call from is_available(),
        # so list_models returns a curated hint list and the user can
        # type any model id into the chat box.
        return []

    def _headers(self) -> dict:
        return {"Authorization": f"Bearer {self.api_key}"}

    def chat(self, req: ChatRequest) -> ChatResponse:
        model = req.model or self.default_model
        if not model:
            return ChatResponse(
                content=f"({self.name}: no model specified)",
                model="", provider=self.name,
            )
        body = {
            "model": model,
            "messages": [m.to_dict() for m in req.messages],
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        }
        if req.stop:
            body["stop"] = req.stop

        status, data = _http_post_json(
            f"{self.base_url}/chat/completions",
            body, self._headers(), self.timeout_s,
        )
        if status >= 400:
            err = data.get("error", data)
            msg = err.get("message", err) if isinstance(err, dict) else err
            return ChatResponse(
                content=f"({self.name} http {status}: {msg})",
                model=model, provider=self.name, raw=data,
            )

        try:
            choice = data["choices"][0]
            content = choice.get("message", {}).get("content", "")
        except (KeyError, IndexError):
            content = f"({self.name}: empty response)"

        usage = data.get("usage", {})
        return ChatResponse(
            content=content,
            model=data.get("model", model),
            provider=self.name,
            usage={
                "prompt_tokens": usage.get("prompt_tokens", 0),
                "completion_tokens": usage.get("completion_tokens", 0),
                "total_tokens": usage.get("total_tokens", 0),
            },
            raw=data,
        )


class OpenAIProvider(OpenAICompatProvider):
    name = "openai"
    default_base_url = "https://api.openai.com/v1"
    default_model = "gpt-4o-mini"
    key_help_url = "https://platform.openai.com/api-keys"


class OpenRouterProvider(OpenAICompatProvider):
    name = "openrouter"
    default_base_url = "https://openrouter.ai/api/v1"
    default_model = "deepseek/deepseek-chat-v3-flash"
    key_help_url = "https://openrouter.ai/settings/keys"


class GrokProvider(OpenAICompatProvider):
    name = "grok"
    default_base_url = "https://api.x.ai/v1"
    default_model = "grok-2-mini"
    key_help_url = "https://console.x.ai/"


class DeepSeekProvider(OpenAICompatProvider):
    name = "deepseek"
    default_base_url = "https://api.deepseek.com/v1"
    default_model = "deepseek-chat"
    key_help_url = "https://platform.deepseek.com/api_keys"


class ZAIProvider(OpenAICompatProvider):
    """Z.AI / Zhipu GLM. Has both international (.com) and domestic
    (.cn) endpoints; default to international."""
    name = "zai"
    default_base_url = "https://api.z.ai/v1"
    default_model = "glm-4.5-flash"
    key_help_url = "https://z.ai/manage-apikey/apikey-list"