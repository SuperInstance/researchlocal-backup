"""providers/local_file.py — workspace RAG as a 'model' provider.

This is the cheapest possible provider: instead of calling an LLM,
it greps the workspace analyses/ folder for keywords. Always available.
Useful as the absolute floor of the fallback chain (chat_offline) and
as a sanity check that the agent has SOMETHING to say even with no
network and no local model.

Not 'intelligent' — but it is a real answer that grounds the agent
in its own history. The granite model on top of this provider can
synthesize the matches into coherent prose.
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Optional

from .base import (
    ModelProvider, ChatRequest, ChatResponse,
)


class LocalFileProvider(ModelProvider):
    name = "local_file"

    def __init__(self, config: Optional[dict] = None):
        config = config or {}
        # workspace root — defaults to sibling tzpro-agent-data
        ws = config.get("workspace_root")
        if ws:
            self.workspace = Path(ws)
        else:
            # default: ../tzpro-agent-data/vessels/fv-eileen
            self.workspace = (
                Path(__file__).resolve().parent.parent
                / "tzpro-agent-data" / "vessels" / "fv-eileen"
            )

    def is_available(self) -> bool:
        return self.workspace.exists()

    def list_models(self) -> list[str]:
        # This provider doesn't really have 'models', but the UI wants
        # something to show. We expose the strategies it can use.
        return [
            "workspace_grep",
            "workspace_keyword",
        ]

    def _gather_corpus(self) -> list[Path]:
        # Look in: analyses/, notes/, captures/ (markdown sidecars),
        # and any *.md anywhere in the vessel folder.
        roots = []
        for sub in ("analyses", "notes", "cabins/notes"):
            d = self.workspace / sub
            if d.exists():
                roots.extend(d.rglob("*.md"))
        # also *.json analysis sidecars
        for sub in ("analyses",):
            d = self.workspace / sub
            if d.exists():
                roots.extend(d.rglob("*.json"))
        return roots

    @staticmethod
    def _score(text: str, query_terms: list[str]) -> int:
        text_l = text.lower()
        return sum(text_l.count(t.lower()) for t in query_terms)

    def chat(self, req: ChatRequest) -> ChatResponse:
        # Use the last user message as the query.
        user_msg = ""
        for m in reversed(req.messages):
            if m.role == "user":
                user_msg = m.content
                break
        if not user_msg:
            return ChatResponse(
                content="(local_file: no user query in messages)",
                provider=self.name,
            )

        # Tokenize: drop punctuation, lowercase, keep words >= 3 chars.
        terms = [t for t in re.findall(r"[A-Za-z0-9_\-]+", user_msg) if len(t) >= 3]
        if not terms:
            return ChatResponse(
                content="(local_file: query had no searchable terms)",
                provider=self.name,
            )

        corpus = self._gather_corpus()
        scored: list[tuple[int, Path, str]] = []
        for p in corpus:
            try:
                txt = p.read_text(encoding="utf-8", errors="replace")
            except Exception:
                continue
            s = self._score(txt, terms)
            if s > 0:
                scored.append((s, p, txt))

        scored.sort(key=lambda x: x[0], reverse=True)
        top = scored[:5]

        if not top:
            return ChatResponse(
                content=(
                    f"(local_file: no workspace matches for terms {terms!r}. "
                    f"Searched {len(corpus)} files.)"
                ),
                provider=self.name,
            )

        # Build a 'response' that lists the top hits with a brief excerpt.
        # This is intentionally not natural language — the calling LLM
        # (or the human reading raw) is expected to interpret.
        lines = [f"[local_file: {len(top)} of {len(corpus)} matches]"]
        for score, path, txt in top:
            rel = path.relative_to(self.workspace) if path.is_relative_to(self.workspace) else path
            # first 240 chars of file
            snippet = " ".join(txt.split())[:240]
            lines.append(f"- ({score}) {rel}: {snippet}{'...' if len(snippet) >= 240 else ''}")
        return ChatResponse(
            content="\n".join(lines),
            model="workspace_grep",
            provider=self.name,
            raw={"match_count": len(scored), "corpus_size": len(corpus)},
        )