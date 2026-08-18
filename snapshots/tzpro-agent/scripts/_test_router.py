"""Test agent_router end-to-end."""
from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from agent_router import AgentRouter, Task
from providers import ChatMessage


r = AgentRouter()


print("== chat_quick (default = deepinfra disabled, falls back to ollama) ==")
resp = r.chat(Task.CHAT_QUICK, [ChatMessage("user", "Reply with one short sentence about being a boat-agent.")])
print(f"  provider: {resp.provider}")
print(f"  model:    {resp.model}")
print(f"  content:  {resp.content.strip()[:200]}")


print("\n== chat_quick forcing local_file by editing routing on the fly ==")
import vessel_config, json
cfg = vessel_config.load(force_reload=True)
# put local_file first for chat_quick to prove routing works
cfg["task_routing"]["chat_quick"] = [{"provider": "local_file"}]
vessel_config._cache = cfg  # mutate cache directly for the test
import agent_router
agent_router._cache = None    # type: ignore  # force AgentRouter to re-read
resp2 = r.chat(Task.CHAT_QUICK, [ChatMessage("user", "echogram sounder ketchikan")])
print(f"  provider: {resp2.provider}")
print(f"  content:  {resp2.content.strip()[:200]}")

# restore
cfg["task_routing"]["chat_quick"] = [
    {"provider": "deepinfra", "model": "deepseek-ai/DeepSeek-V3-Flash", "max_tokens": 800},
    {"provider": "ollama", "model": "granite4.1:8b", "max_tokens": 800},
    {"provider": "local_file"},
]
vessel_config._cache = cfg


print("\n== embed (ollama nomic-embed-text) ==")
e = r.embed(["hello world", "boat agent"], model="nomic-embed-text:latest")
print(f"  provider: {e.provider}")
print(f"  dim:      {e.dim}")
print(f"  vectors:  {len(e.vectors)}")


print("\n== all done ==")