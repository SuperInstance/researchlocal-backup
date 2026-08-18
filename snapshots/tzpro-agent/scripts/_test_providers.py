"""Test the provider abstraction end-to-end."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from providers import (
    get_provider, available_providers,
    ChatRequest, ChatMessage,
)


def test(name, fn):
    try:
        fn()
        print(f"  [OK] {name}")
    except Exception as e:
        print(f"  [FAIL] {name}: {type(e).__name__}: {e}")


print("== provider availability ==")
for n in ["ollama", "deepinfra", "openai", "openrouter",
          "grok", "deepseek", "zai", "browser_native", "local_file"]:
    p = get_provider(n)
    print(f"  {n:18s}  available={p.is_available()}  models={p.list_models()[:3]}")


print("\n== local_file provider (always available) ==")
lf = get_provider("local_file")
req = ChatRequest(messages=[
    ChatMessage("system", "You are a helpful captain's log analyst."),
    ChatMessage("user", "Show me anything about echogram sounder."),
])
resp = lf.chat(req)
print(f"  response preview: {resp.content[:200]}")
print(f"  provider={resp.provider}, model={resp.model}")


print("\n== ollama chat (real call) ==")
o = get_provider("ollama")
if o.is_available():
    req = ChatRequest(messages=[
        ChatMessage("user", "Reply with one sentence: what is 2+2?"),
    ], model="granite4.1:8b", max_tokens=60)
    resp = o.chat(req)
    print(f"  model={resp.model}")
    print(f"  usage={resp.usage}")
    print(f"  content: {resp.content.strip()[:300]}")
else:
    print("  ollama not running, skipping")


print("\n== ollama embed ==")
if o.is_available():
    emb = o.embed(__import__('providers').EmbedRequest(texts=["hello world", "goodbye"]))
    print(f"  dim={emb.dim}, vectors returned={len(emb.vectors)}")
    print(f"  first 5 of v0: {emb.vectors[0][:5]}")


print("\n== available_providers() ==")
for p in available_providers():
    print(f"  {p.name}")


print("\n== all tests done ==")