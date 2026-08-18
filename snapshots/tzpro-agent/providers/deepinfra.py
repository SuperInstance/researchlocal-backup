"""providers/deepinfra.py — DeepInfra OpenAI-compatible API.

DeepInfra is the recommended default cloud provider for Phase 1
because it has a generous free tier, supports many open models
(DeepSeek, Qwen, Llama, Llava for vision, Whisper for STT, etc.)
and prices are low. Casey plans to use it.

Their /v1/chat/completions endpoint is OpenAI-compatible, but we
keep this as a separate class so we can:
  - override is_available() to also check the key is non-empty
    (the base OpenAICompatProvider already does this, but DeepInfra
    has a special 'free inference' token system we may use later).
  - pre-populate a sensible default model list for the UI.
"""
from __future__ import annotations

from typing import Optional

from .openai_compat import OpenAICompatProvider, _http_post_json
from .base import ChatRequest, ChatResponse


class DeepInfraProvider(OpenAICompatProvider):
    name = "deepinfra"
    default_base_url = "https://api.deepinfra.com/v1"
    # cheap + smart + good at pattern analysis (Casey's recommendation)
    default_model = "deepseek-ai/DeepSeek-V3-Flash"
    key_help_url = "https://deepinfra.com/dash/api_keys"

    # A curated set of model ids the UI can show in the dropdown.
    # These are the names we expect most to use during Phase 1.
    SUGGESTED_MODELS = [
        ("deepseek-ai/DeepSeek-V3-Flash", "DeepSeek V3 Flash — fast, cheap, good for pattern analysis"),
        ("deepseek-ai/DeepSeek-V3",       "DeepSeek V3 — slower, deeper reasoning"),
        ("Qwen/Qwen3-Next-80B-A3B-Instruct", "Qwen3 80B — diverse perspective"),
        ("meta-llama/Llama-3.3-70B-Instruct", "Llama 3.3 70B — strong general"),
        ("google/gemini-2.0-flash-001",   "Gemini 2.0 Flash — strong multimodal"),
        ("openai/gpt-4o-mini",            "GPT-4o mini — well-rounded"),
    ]

    def list_models(self) -> list[str]:
        return [m[0] for m in self.SUGGESTED_MODELS]

    def suggested_models(self) -> list[tuple[str, str]]:
        """Returns (id, description) pairs for the UI dropdown."""
        return list(self.SUGGESTED_MODELS)

    def chat(self, req: ChatRequest) -> ChatResponse:
        # DeepInfra's chat completions work the same as OpenAI's,
        # so we just delegate to the parent implementation.
        return super().chat(req)