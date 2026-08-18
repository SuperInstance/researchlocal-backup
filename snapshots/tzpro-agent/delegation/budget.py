"""budget.py -- token-budget awareness for the orchestrator.

# ref: docs/BOOTCAMP.md rule 8 (write handoff when context gets heavy)
# ref: docs/BATON_PASS_2026-07-23.md (collapse at 80k API, 1.4M local)

We track an *estimate* of how much context we are carrying. The actual
API-reported number comes back per-call, but we need an independent
estimate so the harness can decide what to do BEFORE we get a 400-error.

The tiers map to specific behaviors -- the yoke, not the prompt:

    CRYSTAL  < 25%       do anything, explore freely
    CLEAR    25-50%      normal work, start writing handoff breadcrumbs
    CLOUDY   50-75%      commit more often, prefer delegation, smaller prompts
    DARK     75-90%      STOP new exploration; commit + write handoff NOW
    NIGHT    >= 90%      refuse to start new work; only finish + handoff

The whole point is: when the model reaches for the wrong tool at the
wrong tier, the harness refuses -- not the prompt. The model is the
yoke; the harness moves the yoke.

# see: docs/PLANS/current.md for the plan this budget defends
"""
from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, field, asdict
from enum import Enum
from pathlib import Path
from typing import Optional


class Tier(str, Enum):
    CRYSTAL = "CRYSTAL"   # < 25%
    CLEAR = "CLEAR"       # 25-50%
    CLOUDY = "CLOUDY"     # 50-75%
    DARK = "DARK"         # 75-90%
    NIGHT = "NIGHT"       # >= 90%


# Tier ordering for comparisons
_ORDER = [Tier.CRYSTAL, Tier.CLEAR, Tier.CLOUDY, Tier.DARK, Tier.NIGHT]


@dataclass
class Budget:
    """Estimated context budget state.

    api_limit is the wall -- 80_000 for the orchestrator's main model.
    local_estimate is what we *think* we're carrying, in tokens. We
    keep it conservative (assume local is ~17x api-reported because
    that ratio held during the 2026-07-23 collapse).

    The state is persisted to .tzpro-agent/budget.json so the harness
    can recover it after a /clear. The persisted file IS the memory;
    the in-process object is just a view.
    """

    api_limit: int = 80_000
    api_reported: int = 0
    local_estimate: int = 0
    last_call_at: float = field(default_factory=time.time)
    history: list[dict] = field(default_factory=list)

    # -- persistence ----------------------------------------------------

    @property
    def state_path(self) -> Path:
        root = Path(os.environ.get("TZPRO_WORKSPACE", r"C:\Users\casey\tzpro-agent"))
        return root / ".tzpro-agent" / "budget.json"

    def save(self) -> None:
        self.state_path.parent.mkdir(parents=True, exist_ok=True)
        self.state_path.write_text(
            json.dumps(asdict(self), indent=2),
            encoding="utf-8",
        )

    @classmethod
    def load(cls) -> "Budget":
        p = (
            Path(os.environ.get("TZPRO_WORKSPACE", r"C:\Users\casey\tzpro-agent"))
            / ".tzpro-agent" / "budget.json"
        )
        if not p.exists():
            return cls()
        try:
            data = json.loads(p.read_text(encoding="utf-8"))
            # Trim history to last 50 entries to keep this file small
            if "history" in data and len(data["history"]) > 50:
                data["history"] = data["history"][-50:]
            return cls(**data)
        except Exception:
            # Corrupted file -- start fresh, keep a marker
            return cls()

    # -- recording ------------------------------------------------------

    def record(self, *, api_tokens: int, local_tokens: int) -> None:
        """Called after each LLM call.

        api_tokens: what the API reported (may be 0 if we don't know).
        local_tokens: our local estimate (often 0 too; the orchestrator
            harness is the one maintaining it).
        """
        if api_tokens:
            self.api_reported = api_tokens
        if local_tokens:
            self.local_estimate = local_tokens
        self.last_call_at = time.time()
        self.history.append(
            {
                "t": self.last_call_at,
                "api": self.api_reported,
                "local": self.local_estimate,
                "tier": self.tier().value,
            }
        )
        self.save()

    # -- tier logic -----------------------------------------------------

    def pct(self) -> float:
        """Conservative percent used. We use whichever number is higher
        between api_reported and local_estimate (the local number
        tracks reality better past the collapse point)."""
        api_pct = self.api_reported / self.api_limit if self.api_limit else 0
        local_pct = self.local_estimate / self.api_limit if self.api_limit else 0
        return max(api_pct, local_pct)

    def tier(self) -> Tier:
        p = self.pct()
        if p < 0.25:
            return Tier.CRYSTAL
        if p < 0.50:
            return Tier.CLEAR
        if p < 0.75:
            return Tier.CLOUDY
        if p < 0.90:
            return Tier.DARK
        return Tier.NIGHT

    def at_least(self, t: Tier) -> bool:
        return _ORDER.index(self.tier()) >= _ORDER.index(t)

    # -- behavior gates -------------------------------------------------

    def can(self, action: str) -> bool:
        """The yoke. Specific actions gated by tier.

        The point: don't argue with the model in the prompt. Just
        refuse the action at the API call site and let the model try
        something else. If it tries the same thing 3 times, that's a
        signal the model is stuck -- time to surface to the user.

        Actions:
          explore       - read a new file we haven't read before
          delegate      - spawn a Claude Code sub-session
          big_edit      - rewrite a file >200 lines or add new module
          commit        - git commit (always allowed; preserves work)
          write_handoff - write a handoff doc (always allowed; this
                          IS the safety mechanism)
          start_new     - begin work on a new task vs continuing
                          existing one (refused in DARK/NIGHT)
        """
        t = self.tier()
        if action in ("commit", "write_handoff"):
            return True
        if t == Tier.NIGHT:
            return action in ("commit", "write_handoff")
        if t == Tier.DARK:
            return action in ("explore", "delegate", "commit", "write_handoff")
        if t == Tier.CLOUDY:
            # CLOUDY: still allowed to do anything, but the harness
            # will LOG a soft warning suggesting delegation. The
            # model reaches for the right tool on its own.
            return True
        # CRYSTAL or CLEAR -- all gates open
        return True

    def why_not(self, action: str) -> Optional[str]:
        """Diagnostic: why was this action refused?"""
        if self.can(action):
            return None
        t = self.tier()
        return (
            f"action={action!r} refused at tier={t.value} "
            f"(api={self.api_reported}/{self.api_limit}, "
            f"local~{self.local_estimate}); "
            f"see docs/BOOTCAMP.md rule 8 -- write a handoff doc "
            f"or commit current work, then /clear to reset"
        )


# -- module-level singleton -------------------------------------------------

_BUDGET: Optional[Budget] = None


def get() -> Budget:
    global _BUDGET
    if _BUDGET is None:
        _BUDGET = Budget.load()
    return _BUDGET


def reset() -> None:
    """Drop the cached Budget; force reload from disk."""
    global _BUDGET
    _BUDGET = None


def gate(action: str) -> None:
    """Raise if action is not allowed at current tier.

    Use as a one-liner at the top of any function that does
    something potentially expensive:

        from delegation.budget import gate
        gate("big_edit")  # raises TierRefused if DARK/NIGHT
    """
    b = get()
    if not b.can(action):
        raise TierRefused(b.why_not(action) or "refused")


class TierRefused(Exception):
    """Raised when an action is gated by current budget tier.

    Catch this in the orchestrator loop; do NOT try to work around it
    by editing the prompt. Either:
      - commit what you have and /clear to reset
      - delegate the work to Claude Code (cheaper in context)
      - write a handoff doc and end the session
    """


if __name__ == "__main__":
    # quick self-test
    b = Budget(api_limit=80_000, api_reported=20_000, local_estimate=20_000)
    assert b.tier() == Tier.CLEAR
    b.record(api_tokens=42_000, local_tokens=42_000)
    assert b.tier() == Tier.CLOUDY
    print(f"budget self-test: tier={b.tier().value} pct={b.pct():.2f}")
    print(f"can('big_edit')={b.can('big_edit')}  why={b.why_not('big_edit') or '-'}")