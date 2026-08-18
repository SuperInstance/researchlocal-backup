# Navigation Test Result

**Generated:** 2026-07-23
**Agent:** Fresh orchestrator (post-onboard)

---

## SUMMARY

This project is **tzpro-agent**: a local-first capture-and-analysis system for F/V Eileen, a commercial fishing vessel. The system watches a TZ Pro sounder screen, captures frames on cadence, indexes by time/location, and serves them via a LAN web dashboard with chatbot. Architecture follows a "repo IS the agent" ship metaphor (hull/slip/chandlery/log), with Phase 1 focus on capture, tray app, LAN dashboard, and provider abstraction.

## CURRENT STATE

**Working (8/9 doctor checks):**
- Bridge alive on :6006 (PID 11940, 8 sats, fix_q=1)
- Ollama running with 4 models (granite4.1:8b, gemma4:12b, nomic-embed-text, qwen3:4b)
- Vault DPAPI + AES-GCM round-trip verified
- Dashboard on :8090 serving vessel=F/V Eileen with ollama+local_file providers
- Capture daemon: STOPPED (expected, can be started via tray)
- Delegation harness built (budget, lesson_plan, onboard, claude_runner)

**Broken / Incomplete:**
- `state:jsonl` check FAILING - last write 14454s ago (~4 hours)
- **Working tree has significant uncommitted work** - 8 modified files (cascade/, doctor.py) + 25 untracked files (agent_router.py, capture_daemon.py, dashboard/, tray_app.py, etc.)
- `state:jsonl` check incorrectly fails when boat is DOCKED (per BOOTCAMP.md rule 5, stationary at dock is valid state)

**Git Status:**
- Latest commit: `564dc82 feat(delegation): token-budget-aware harness`
- The Phase 1 work (capture_daemon, tray_app, dashboard, doctor extensions) is **uncommitted** in the working tree

## NEXT ACTION

**Commit the Phase 1 work immediately.** The working tree carries >4 hours of uncommitted work (capture_daemon.py, tray_app.py, dashboard/, doctor.py modifications, delegation harness, assets). Per BOOTCAMP.md rule 1 ("Commit early, commit often") and BATON_PASS recommendation, this is the blocking action before any new work. The previous session collapsed due to failure to commit; this must not repeat.

After commit, the next actions are:
1. Fix `state:jsonl` doctor check to accept DOCKED state as valid (BOOTCAMP rule 5)
2. Push to remote if network cooperates
3. Verify desktop shortcut works end-to-end

## EVIDENCE

```
Doctor Output:
============================================================
  tzpro-agent doctor: 8/9 healthy
  1 failing:
      - state:jsonl: last write 14454s ago (state stream appears stalled)
============================================================
```

```
Git Status (short):
M .gitignore
 M cascade/daemon.py
 M cascade/daily_loop.py
 M cascade/decaminute_loop.py
 M cascade/hourly_loop.py
 M cascade/retention.py
 M doctor.py
?? .last_nmea_heartbeat
?? agent_router.py
?? assets/icon-shortcut-256.png
?? assets/icon-source.png
?? assets/icon-tray-16.png
?? assets/icon-tray-32.png
?? assets/icon-tray-64.png
?? capture_daemon.py
?? captures/
?? cascade/test_sidecar_only.py
?? cascade_out/
?? dashboard.py
?? dashboard/
?? delegation/tasks/
?? logs/
?? memory/
?? providers/
?? roster/
?? schema/
?? scripts/_make_shortcut.ps1
?? scripts/_stop_tray.bat
?? scripts/_test_capture_daemon.py
?? scripts/_test_phase1_e2e.py
?? scripts/_test_providers.py
?? scripts/_test_router.py
?? scripts/_test_tray_app.py
?? scripts/pull_vision_models.ps1
?? start_capture_tray.bat
?? tray_app.py
?? vessel.json
?? vessel_config.py
?? vessel_state.jsonl
```

```
Git Log (last 5):
564dc82 feat(delegation): token-budget-aware harness (budget, lesson_plan, onboard, claude_runner)
dd23dc5 docs(bootcamp): BATON_PASS for 2026-07-23 collapse + operating rules
89593e6 feat(vault+doctor+roadmap): DPAPI vault with doctor health check, ROADMAP, tray icon assets
02315c6 feat(companion): cascade -> ship-log-search bridge with tests
26c24aa fix(capture_v3): guard against None SOG/COG when NMEA fix unavailable
```
