"""delegation -- orchestrator's coordination package.

Modules in this package:
    budget       -- token-budget awareness, tier gates (yoke-move)
    lesson_plan  -- daily/weekly plan storage that auto-evolves
    onboard      -- zero-shot navigator for fresh agents
    claude_runner-- bridge to local Claude Code sub-sessions
    next_action  -- deterministic "what should I do next?" tool

# ref: docs/BOOTCAMP.md rule 8 (write handoff when context gets heavy)
# ref: docs/AGENT_OPERATING_MODEL.md (the git-agent paradigm)
# see: docs/NAVIGATION.md (where these tools are wired into the cockpit)
"""