"""providers/ollama.py — local model server at 127.0.0.1:11434.

No auth, no key. Always the first thing to try when the boat laptop
is on. If ollama isn't running, this provider reports is_available()
== False and the agent falls back to a cloud provider or local_file.

Models installed on Eileen's laptop as of 2026-07-23:
  - granite4.1:8b        (5.3 GB)  default local chat
  - gemma4:12b           (7.6 GB)  heavier local reasoning
  - nomic-embed-text     (274 MB)  embeddings
  - qwen3:4b             (2.5 GB)  fast offline fallback

Note: we use the HTTP /api/chat and /api/embeddings endpoints (not the
newer /v1/chat/completions OpenAI-compat layer) because they're
stable across ollama versions and don't require extra config.
"""
from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Optional

from .base import (
    ModelProvider, ChatRequest, ChatResponse,
    EmbedRequest, EmbedResponse,
)


DEFAULT_BASE = "http://127.0.0.1:11434"


class OllamaProvider(ModelProvider):
    name = "ollama"

    def __init__(self, config: Optional[dict] = None):
        config = config or {}
        self.base = config.get("base_url", DEFAULT_BASE).rstrip("/")
        self.timeout_s = float(config.get("timeout_s", 120))

    # ---- availability & discovery ---------------------------------------

    def is_available(self) -> bool:
        try:
            with urllib.request.urlopen(f"{self.base}/api/tags",
                                         timeout=2) as resp:
                return resp.status == 200
        except Exception:
            return False

    def list_models(self) -> list[str]:
        try:
            with urllib.request.urlopen(f"{self.base}/api/tags",
                                         timeout=5) as resp:
                data = json.loads(resp.read().decode("utf-8"))
        except Exception:
            return []
        return [m.get("name", "") for m in data.get("models", []) if m.get("name")]

    # ---- chat -----------------------------------------------------------

    def chat(self, req: ChatRequest) -> ChatResponse:
        model = req.model or "granite4.1:8b"
        body = {
            "model": model,
            "messages": [m.to_dict() for m in req.messages],
            "stream": False,
            "options": {
                "temperature": req.temperature,
                "num_predict": req.max_tokens,
            },
        }
        if req.stop:
            body["options"]["stop"] = req.stop

        try:
            with urllib.request.urlopen(
                urllib.request.Request(
                    f"{self.base}/api/chat",
                    data=json.dumps(body).encode("utf-8"),
                    headers={"Content-Type": "application/json"},
                    method="POST",
                ),
                timeout=self.timeout_s,
            ) as resp:
                data = json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:
            return ChatResponse(
                content=f"(ollama http {e.code}: {e.reason})",
                model=model, provider=self.name,
            )
        except Exception as e:
            return ChatResponse(
                content=f"(ollama error: {type(e).__name__}: {e})",
                model=model, provider=self.name,
            )

        return ChatResponse(
            content=data.get("message", {}).get("content", ""),
            model=data.get("model", model),
            provider=self.name,
            usage={
                "prompt_tokens": data.get("prompt_eval_count", 0),
                "completion_tokens": data.get("eval_count", 0),
                "total_tokens": (
                    data.get("prompt_eval_count", 0) +
                    data.get("eval_count", 0)
                ),
            },
            raw=data,
        )

    # ---- embeddings -----------------------------------------------------

    def embed(self, req: EmbedRequest) -> EmbedResponse:
        model = req.model or "nomic-embed-text:latest"
        out_vectors: list[list[float]] = []
        # ollama's /api/embeddings takes one prompt at a time
        for t in req.texts:
            body = {"model": model, "prompt": t}
            try:
                with urllib.request.urlopen(
                    urllib.request.Request(
                        f"{self.base}/api/embeddings",
                        data=json.dumps(body).encode("utf-8"),
                        headers={"Content-Type": "application/json"},
                        method="POST",
                    ),
                    timeout=self.timeout_s,
                ) as resp:
                    data = json.loads(resp.read().decode("utf-8"))
            except Exception as e:
                # continue with zero vector for failed text
                out_vectors.append([0.0] * (out_vectors[0] if out_vectors else [768]) and 768 or 768)
                continue
            out_vectors.append(data.get("embedding", []))

        dim = len(out_vectors[0]) if out_vectors else 0
        return EmbedResponse(
            vectors=out_vectors, model=model, provider=self.name, dim=dim,
        )