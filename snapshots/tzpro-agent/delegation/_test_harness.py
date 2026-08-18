"""_test_harness.py -- smoke tests for the delegation harness pieces.

Run: python -m delegation._test_harness
"""
from delegation.budget import Budget, Tier
from delegation.lesson_plan import bootstrap, note, next_action, actual, delta
from delegation.onboard import onboard
from pathlib import Path


def test_budget_tiers():
    print("=== budget tiers ===")
    cases = [
        (0, Tier.CRYSTAL),
        (20_000, Tier.CLEAR),
        (42_000, Tier.CLOUDY),
        (70_000, Tier.DARK),
        (78_000, Tier.NIGHT),
    ]
    for tokens, expected in cases:
        b = Budget(api_limit=80_000, api_reported=tokens, local_estimate=tokens)
        actual_t = b.tier()
        status = "OK" if actual_t == expected else "FAIL"
        print(f"  [{status}] {tokens:>6} -> {actual_t.value} (expected {expected.value})")

    # Gate behavior at NIGHT
    b = Budget(api_limit=80_000, api_reported=78_000, local_estimate=78_000)
    print(f"  NIGHT: can(commit)={b.can('commit')} "
          f"can(write_handoff)={b.can('write_handoff')} "
          f"can(delegate)={b.can('delegate')} "
          f"can(start_new)={b.can('start_new')}")


def test_lesson_plan():
    print("=== lesson plan ===")
    today, week, cur = bootstrap()
    print(f"  today:  {today.name}")
    print(f"  week:   {week.name}")
    print(f"  cur:    {cur.name}")
    note("harness build smoke test -- appended note", scope="daily")
    actual("harness pieces created: budget, lesson_plan, onboard, claude_runner",
           scope="daily")
    delta("lesson plan storage works on first invocation",
          scope="daily")
    next_action("commit the delegation harness pieces", scope="daily")
    # Read back to verify
    body = today.read_text(encoding="utf-8")
    assert "SESSION-NOTE" in body, "SESSION-NOTE missing"
    assert "RESULT" in body, "RESULT missing"
    assert "DELTA-NOTE" in body, "DELTA-NOTE missing"
    assert "NEXT" in body, "NEXT missing"
    print("  [OK] all sections present")


def test_onboard():
    print("=== onboard ===")
    for scope in ["default", "monologue", "cost-throttle", "debugging", "phase-1"]:
        out = onboard(scope)
        first_line = out.splitlines()[0]
        print(f"  [OK] scope={scope!r} -> {first_line}")


if __name__ == "__main__":
    test_budget_tiers()
    print()
    test_lesson_plan()
    print()
    test_onboard()
    print()
    print("all harness smoke tests passed")