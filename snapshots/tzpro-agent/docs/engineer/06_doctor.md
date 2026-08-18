# Doctor — `doctor.py`

**File:** `doctor.py` (~500 lines)
**Priority:** Operational — the canonical health check.
**Owner:** tzpro-agent.

## What It Does

A single-file health-check + auto-repair tool. Runs a battery of
checks against every subsystem and reports pass/fail. The
`fix --yes` subcommand attempts to repair the failures it can.

```bash
python doctor.py check            # report only
python doctor.py check --no-color # plain text
python doctor.py fix              # report + attempt repair
python doctor.py fix --yes        # skip confirmation prompt
```

## Check Battery (`run_checks`, `doctor.py`)

| Check | Function | What it verifies |
|---|---|---|
| `bridge:tcp:6006` | `check_bridge_tcp_port` | TCP :6006 has a listener |
| `bridge:http:8654` | `check_bridge_http_api` | HTTP :8654 returns `/health` |
| `bridge:heartbeat` | `check_bridge_heartbeat_fresh` | `.last_nmea_heartbeat` < 15 s old |
| `bridge:serial` | `check_bridge_serial_open` | Some python process has COM6 open |
| `vessel:state_recent` | `check_vessel_state_recent` | `vessel_state.jsonl` written within 30 s |
| `ollama:11434` | `check_ollama_running` | Ollama local LLM server reachable |
| `vault:roundtrip` | `check_vault_roundtrip` | Vault decrypts a known marker |
| `dashboard:8090` | `check_dashboard_alive` | HTTP :8090 reachable |
| `capture:daemon` | `check_capture_daemon` | Stampfile exists + daemon PID alive |

Output is one line per check, color-coded:

```
bridge:tcp:6006       OK      (PID 29828, 1 client)
bridge:http:8654      OK      ({"ok": true})
bridge:heartbeat      OK      (0.0s old)
bridge:serial         OK      (COM6 open by PID 29828)
vessel:state_recent   OK      (last write 2.1s ago)
ollama:11434          FAIL    (ConnectionRefused: 127.0.0.1:11434)
vault:roundtrip       OK      (decrypted in 14ms)
dashboard:8090        OK      (200 OK in 18ms)
capture:daemon        OK      (daemon=OK child=OK tzpro=OK(7192))

9/9 healthy
```

## Auto-Repair (`cmd_fix`, `doctor.py`)

Each check that fails is followed by an attempted repair:

| Failure | Repair |
|---|---|
| `bridge:*` all fail | `_start_bridge_windows()` — relaunches `nmea_bridge.py` |
| `dashboard:8090` | `_start_dashboard_windows()` — relaunches `dashboard.py` |
| `capture:daemon` | `_stop_capture_daemon()` then start fresh |

Repair steps that can't be safely automated (e.g. ollama not running)
print a hint instead.

## Health-Probe Helpers

| Function | Purpose |
|---|---|
| `_http_get_json(path)` | GET a URL, return parsed JSON or `None` |
| `_process_listening_on(port)` | Find a PID listening on `port` |
| `_pid_has_command_substring(pid, substring)` | Verify the listener is *ours* |
| `_heartbeat_age_seconds()` | Age of `.last_nmea_heartbeat` |
| `_parse_heartbeat_iso()` | ISO timestamp from the heartbeat file |

These are reused by `dashboard.py` and the tray's doctor-check
action.

## Adding a New Check

1. Add a `check_<name>() -> CheckResult` function. Return a
   `CheckResult(name, ok, message)` — keep the message short.
2. Append it to `run_checks()` (`doctor.py`).
3. Add a repair step to `cmd_fix()` if the failure is auto-fixable.
4. Update `docs/engineer/06_doctor.md` with the new check.

`CheckResult.render(color=True)` handles the formatting.

## Verified Line Numbers (as of `b64c2ef`)

- `CheckResult`: `doctor.py` top
- HTTP helpers: ~`doctor.py:30-90`
- Check functions: search `def check_`
- `run_checks`: ~middle of `doctor.py`
- `cmd_check` / `cmd_fix`: search `def cmd_`
- `main`: search `def main`

## Common Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| All bridge checks fail | Bridge process dead | `fix --yes` |
| `ollama:11434 FAIL` | Ollama service stopped | `ollama serve` in another shell |
| `vault:roundtrip FAIL` | Vault corrupted or wrong Windows user | DO NOT cross-user move vault.dat; restore from backup of same machine |
| `capture:daemon FAIL` | Daemon crashed or stampfile stale | `fix --yes` |
| False positive on `bridge:serial` | Another app (e.g. Serial Monitor) holds COM6 | Close the other app |

## Related Docs

- All other `engineer/*.md` docs — each subsystem has a doctor check
- `docs/TROUBLESHOOTING.md` — symptom-driven cookbook (legacy)
- `fix_priority0.bat` — doctor knows about this script and invokes it