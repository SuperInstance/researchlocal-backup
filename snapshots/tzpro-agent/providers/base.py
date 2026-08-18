"""providers/base.py — Provider abstraction for boat-agent LLM calls.

Every model backend (cloud API, local ollama, browser-native, etc.)
implements the ModelProvider interface. The agent picks a provider
per task (chat, vision, embedding, transcription).

The base class is intentionally minimal: just enough to keep all
implementations uniform. Per-provider subclasses add whatever methods
they actually support; unsupported operations raise NotImplementedError
so the agent can fall back to another provider.

Why an interface at all
-----------------------
The whole point of Phase 1 is that the right-panel chat should be
able to talk to ANY provider the captain configures, without us
having to write special-case code for each one. The base interface
is the seam where new providers plug in.

Provider types we expect to see in Phase 1
------------------------------------------
- ollama          : local, free, on the boat laptop
- browser_native  : chrome/edge window.ai or extension (future)
- deepinfra       : cloud, cheap, multi-model
- openai / openrouter / grok / deepseek / z.ai : cloud
- workers_ai      : cloudflare (later phase)
- local_file      : workspace grep/RAG over analyses/
"""
from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, AsyncIterator, Optional


@dataclass
class ChatMessage:
    role: str       # "system" | "user" | "assistant"
    content: str

    def to_dict(self) -> dict:
        return {"role": self.role, "content": self.content}


@dataclass
class ChatRequest:
    messages: list[ChatMessage]
    model: str = ""
    temperature: float = 0.7
    max_tokens: int = 1024
    stop: list[str] = field(default_factory=list)
    # Optional metadata the provider can ignore.
    task: str = ""           # e.g. "chat_quick", "chat_heavy", "analyze_10min"
    session_id: str = ""


@dataclass
class ChatResponse:
    content: str
    model: str = ""
    provider: str = ""
    usage: dict = field(default_factory=dict)  # {prompt_tokens, completion_tokens, total_tokens}
    raw: dict = field(default_factory=dict)


@dataclass
class VisionRequest:
    # The image is always present as bytes + a mime type.
    image_bytes: bytes
    image_mime: str            # "image/png" | "image/jpeg" | ...
    prompt: str
    model: str = ""
    max_tokens: int = 512


@dataclass
class EmbedRequest:
    texts: list[str]
    model: str = ""


@dataclass
class EmbedResponse:
    vectors: list[list[float]]
    model: str = ""
    provider: str = ""
    dim: int = 0


class ModelProvider(ABC):
    """Abstract base class for any model backend.

    Subclasses MUST define:
      - name (str): unique short id used in vessel.json (e.g. "ollama",
                   "deepinfra", "browser_native").

    Subclasses SHOULD override:
      - chat(): text in, text out.
      - vision(): image in + prompt, text out.
      - embed(): texts in, vectors out.
      - transcribe(): audio in, text out (later phase).

    Implementations raise NotImplementedError for methods they don't
    support so the caller can fall back.
    """

    name: str = "base"

    @abstractmethod
    def is_available(self) -> bool:
        """Cheap liveness check. Should not make heavy API calls.
        Returns True if the provider can be used right now.
        """

    @abstractmethod
    def list_models(self) -> list[str]:
        """List model ids this provider currently offers. Used by the
        UI dropdown and the auto-fallback router.
        """

    def chat(self, req: ChatRequest) -> ChatResponse:
        raise NotImplementedError(f"{self.name} does not support chat")

    async def achat(self, req: ChatRequest) -> ChatResponse:
        """Default async wrapper just calls sync chat(). Providers with
        a native async API (e.g. openai, httpx) override this."""
        return self.chat(req)

    def vision(self, req: VisionRequest) -> ChatResponse:
        raise NotImplementedError(f"{self.name} does not support vision")

    def embed(self, req: EmbedRequest) -> EmbedResponse:
        raise NotImplementedError(f"{self.name} does not support embedding")

    def transcribe(self, audio_bytes: bytes, mime: str = "audio/wav") -> str:
        raise NotImplementedError(f"{self.name} does not support transcription")


def get_provider(name: str, config: Optional[dict] = None) -> ModelProvider:
    """Factory: build a provider by name. Lazy-imports so a missing
    optional dependency (e.g. openai) doesn't break providers that
    don't need it.

    config is passed to the provider constructor; provider-specific
    schema lives in the provider's own module.
    """
    from . import ollama as _ollama
    from . import deepinfra as _deepinfra
    from . import openai_compat as _openai_compat
    from . import browser_native as _browser
    from . import local_file as _local

    config = config or {}
    table: dict[str, Any] = {
        "ollama": _ollama.OllamaProvider,
        "deepinfra": _deepinfra.DeepInfraProvider,
        "openai": _openai_compat.OpenAICompatProvider,
        "openrouter": _openai_compat.OpenRouterProvider,
        "grok": _openai_compat.GrokProvider,
        "deepseek": _openai_compat.DeepSeekProvider,
        "zai": _openai_compat.ZAIProvider,
        "browser_native": _browser.BrowserNativeProvider,
        "local_file": _local.LocalFileProvider,
    }
    if name not in table:
        raise ValueError(f"unknown provider: {name}")
    return table[name](config)


def available_providers(config: Optional[dict] = None) -> list[ModelProvider]:
    """Return all providers that report is_available() == True.
    Used by the dashboard's provider picker and the agent's
    fallback router.
    """
    names = [
        "ollama", "deepinfra", "openai", "openrouter", "grok",
        "deepseek", "zai", "browser_native", "local_file",
    ]
    out = []
    for n in names:
        try:
            p = get_provider(n, config)
            if p.is_available():
                out.append(p)
        except Exception:
            pass
    return out