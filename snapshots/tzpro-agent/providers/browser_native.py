"""providers/browser_native.py — Chrome / Edge built-in AI (experimental).

Phase 1 stub. The real version would detect:
  - Chrome's `window.ai` (the Prompt API behind the right flag).
  - Edge's similar API.
  - Browser extensions that expose a chat endpoint (Aider, Continue,
    custom Gemini/GPT sidebars).

We can't actually call a browser from Python without a CDP/Playwright
sidecar. The Phase 1 implementation only does detection (checking
whether the browser claims the capability is registered in a local
config file the user maintains).

Why this matters: if we can harness built-in browser AI, a "light
duty" user can get going with zero API spend. We want the seam in
place even before the implementation is real.

Configuration contract (lives in vessel.json under
providers.browser_native):
  {
    "cdp_url": "http://127.0.0.1:9222",   # Chrome DevTools Protocol
    "extension_endpoint": "http://localhost:..."   # optional
  }
"""
from __future__ import annotations

from typing import Optional

from .base import (
    ModelProvider, ChatRequest, ChatResponse,
)


class BrowserNativeProvider(ModelProvider):
    name = "browser_native"

    def __init__(self, config: Optional[dict] = None):
        config = config or {}
        self.cdp_url = config.get("cdp_url", "")
        self.extension_endpoint = config.get("extension_endpoint", "")

    def is_available(self) -> bool:
        # Phase 1: only available if the user has explicitly configured
        # a CDP endpoint. We don't ping because a CDP-less install
        # shouldn't trigger network errors every cycle.
        return bool(self.cdp_url) or bool(self.extension_endpoint)

    def list_models(self) -> list[str]:
        # We don't know what the browser exposes; let the user type
        # whatever string they want.
        return ["browser_default"]

    def chat(self, req: ChatRequest) -> ChatResponse:
        return ChatResponse(
            content=(
                "(browser_native: not implemented in Phase 1. "
                "Configure cdp_url or extension_endpoint in vessel.json "
                "and the provider will be enabled once Phase 1 R&D lands.)"
            ),
            model=req.model or "browser_default",
            provider=self.name,
        )