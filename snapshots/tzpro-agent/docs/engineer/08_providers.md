# Provider Abstraction — `providers/`

**Package:** `providers/` (7 modules)
**Priority:** Foundation for AI features; not P0/P1 themselves.
**Owner:** tzpro-agent.

## What It Does

A uniform interface for any model backend (local ollama, cloud APIs,
browser-native, local-file RAG). The agent picks a provider per task
based on what's available, what the user enabled in `vessel.json`,
and a fallback chain.

## Module Map

| Module | Purpose |
|---|---|
| `providers/__init__.py` | Re-exports the public API |
| `providers/base.py` | `ModelProvider` ABC + dataclasses + factory |
| `providers/ollama.py` | Local ollama (`http://127.0.0.1:11434`) |
| `providers/deepinfra.py` | DeepInfra cloud API |
| `providers/openai_compat.py` | OpenAI + OpenRouter + Grok + DeepSeek + Z.AI |
| `providers/browser_native.py` | Chrome/Edge `window.ai` (future) |
| `providers/local_file.py` | Workspace grep/RAG over `analyses/` |

All concrete providers share the same interface.

## The Interface (`providers/base.py`)

### Dataclasses

```python
ChatMessage(role: str, content: str)
ChatRequest(messages, model, temperature, max_tokens, stop, task, session_id)
ChatResponse(content, model, provider, usage, raw)
VisionRequest(image_bytes, image_mime, prompt, model, max_tokens)
EmbedRequest(texts, model)
EmbedResponse(vectors, model, provider, dim)
```

### `ModelProvider` ABC

| Method | Required | Returns |
|---|---|---|
| `name: str` | yes (class attr) | short id (`"ollama"`, `"deepinfra"`, …) |
| `is_available() -> bool` | yes | cheap liveness check |
| `list_models() -> list[str]` | yes | model ids this provider offers |
| `chat(req: ChatRequest) -> ChatResponse` | optional | default raises `NotImplementedError` |
| `achat(req) -> ChatResponse` | optional | async wrapper; default delegates to sync |
| `vision(req: VisionRequest) -> ChatResponse` | optional | default raises |
| `embed(req: EmbedRequest) -> EmbedResponse` | optional | default raises |
| `transcribe(audio_bytes, mime) -> str` | optional | default raises |

The pattern: **declare what's not supported, raise on call, fall back at the caller.** This is how `agent_router.py` walks a chain of providers until one succeeds.

## Factory

```python
from providers import get_provider, available_providers

p = get_provider("ollama", config)        # build one
ps = available_providers(config)          # build all that are alive
```

`get_provider()` (`providers/base.py:138-166`) lazy-imports so a
missing optional dependency (e.g. the `openai` package for OpenAI
compatibility) doesn't break providers that don't need it.

## Adding a New Provider

1. Create `providers/<name>.py`.
2. Subclass `ModelProvider`. Set `name`.
3. Implement `is_available`, `list_models`, and any of `chat`,
   `vision`, `embed`, `transcribe` you support.
4. Add the class to the `table` dict in `get_provider()`
   (`providers/base.py:153-163`).
5. Add a section to `vessel.json` (see below).

## Configuration in `vessel.json`

`vessel.json` controls which providers are enabled and per-task
routing. Excerpt:

```json
{
  "providers": {
    "ollama":  {"enabled": true,  "base_url": "http://127.0.0.1:11434"},
    "deepinfra": {"enabled": false, "api_key_secret": "deepinfra"},
    "openai":  {"enabled": false, "api_key_secret": "openai"}
  },
  "task_routing": {
    "chat_quick":   ["ollama", "deepinfra", "openai"],
    "chat_heavy":   ["deepinfra", "openai"],
    "vision":       ["deepinfra", "openai"],
    "embed":        ["ollama", "deepinfra"]
  }
}
```

- `enabled: true` makes the provider a candidate.
- `api_key_secret` is the **name** of a vault secret; the factory
  reads it via `Vault().get(secret_name)` at provider construction.
- `task_routing[my_task]` is an ordered fallback chain; the router
  tries each in turn, stopping on first success.

## Provider Capabilities

| Provider | chat | vision | embed | transcribe |
|---|---|---|---|---|
| ollama (local) | ✅ | ✅ (llava, granite-vision) | ✅ (nomic-embed) | ✅ (whisper) |
| deepinfra | ✅ | ✅ (Gemini 2.0 Flash, etc.) | ✅ | ❌ |
| openai-compat (5 backends) | ✅ | ✅ | ✅ | ❌ |
| browser_native | planned | planned | ❌ | ❌ |
| local_file | RAG-only | ❌ | lexical | ❌ |

## Verified Line Numbers (as of `b64c2ef`)

- Dataclasses: `providers/base.py:35-86`
- ABC: `providers/base.py:89-135`
- Factory table: `providers/base.py:153-163`
- `available_providers`: `providers/base.py:169-…`

## Related Docs

- `docs/engineer/07_vault.md` — where API keys live
- `docs/architecture/PROJECTION_LAYER.md` — where the providers fit
  in the bigger stack
- `docs/phases/2.md` — where the cascade loop fans out across providers