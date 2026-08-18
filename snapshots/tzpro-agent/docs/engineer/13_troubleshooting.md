# Troubleshooting Cookbook

> **Audience:** the captain and any engineer responding to a "the
> system is broken" call. Start with `python doctor.py check` to
> localize the problem, then jump to the matching section.

## Quick Triage

```bash
python doctor.py check
```

The output tells you which subsystem is broken. The check name maps
1:1 to a section below.

| Check name | Section |
|---|---|
| `bridge:tcp:6006`, `bridge:http:8654`, `bridge:heartbeat`, `bridge:serial` | [Bridge](#bridge) |
| `vessel:state_recent` | [Bridge / State File](#bridge) |
| `ollama:11434` | [Ollama](#ollama) |
| `vault:roundtrip` | [Vault](#vault) |
| `dashboard:8090` | [Dashboard](#dashboard) |
| `capture:daemon` | [Capture](#capture) |

---

## Bridge

**Symptoms:** TZ Pro shows no boat position; chart is empty.

### All four bridge checks fail

The bridge is dead.

```powershell
# From C:\Users\casey\tzpro-agent\
powershell -Command "Get-Process python -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like '*nmea_bridge*' } | Stop-Process -Force"
Start-Process pythonw -ArgumentList "nmea_bridge.py --port COM6 --baud 4800" -WorkingDirectory "C:\Users\casey\tzpro-agent\"
```

Or simply: `fix_priority0.bat`.

### `bridge:serial` fails (others pass)

GPS is sending data but the COM port isn't opening in shared mode.
This is almost always the **signed-driver block** in newer TZ Pro
builds. Fix: install com0com, create a virtual pair, point TZ Pro at
one end and the bridge at the other. See `01_nmea_bridge.md`.

### `bridge:heartbeat` stale

Bridge process is up but not reading from COM6. Likely causes:

- GPS receiver unpowered
- Wrong COM port (`--port COM7` instead of `COM6`)
- Wrong baud rate (`--baud 9600` instead of `4800`)

Run `python nmea_bridge.py --diag --diag-seconds 8` to see raw
sentences. If you see NMEA, the bridge is fine; check TZ Pro's
NMEA source config (TCP 127.0.0.1:6006, Protocol NMEA0183).

### Sentences parse but TZ Pro shows no position

TZ Pro's NMEA source config is wrong. In TZ Pro:

- `Initialization → GPS → Source = TCP`
- `Host = 127.0.0.1`
- `Port = 6006`
- `Protocol = NMEA0183`
- `Baud = 4800` (irrelevant for TCP but set anyway)

Restart TZ Pro to apply.

---

## Capture

**Symptoms:** GPS works fine; no captures appearing in
`captures/v3/<today>/`.

### `capture:daemon` shows STOPPED

```bash
python capture_daemon.py status
```

If stampfile missing: start it.

```bash
# Tray → "Start capture", or:
python capture_daemon.py run --auto
```

If stampfile present but daemon not alive: stale stampfile. Stop and start.

```bash
python capture_daemon.py stop
python capture_daemon.py run --auto
```

### Daemon alive but child not running

The daemon should respawn the child automatically. If not, check
`logs/capture_v3.daemon-child.log` for the child's stderr.

Common cause: PowerShell screenshot script needs a logged-in desktop
session. Verify by manually running `screenshot_v3.ps1` — if it
errors, see [DISPLAY6 not capturable](#display6-not-capturable).

### `verify` reports STALE

TZ Pro up, daemon up, child up, but no recent files. This means the
child is running but not producing output.

- Check `logs/capture_v3.daemon-child.log` for the most recent line.
- Check `python doctor.py check` — if `bridge:serial` is also
  failing, fix the bridge first (capture_v3 needs position).
- Verify the day folder exists: `ls captures/v3/`. If empty, the
  bridge is the suspect.

### DISPLAY6 not capturable

`screenshot_v3.ps1` uses `System.Drawing.Graphics.CopyFromScreen`
which requires:

- A logged-in desktop session (RDP doesn't count unless console)
- A second monitor at X=1920 (or update `DISPLAY_OFFSET_X` in
  `capture_v3.py`)

To test manually:

```powershell
powershell -ExecutionPolicy Bypass -File screenshot_v3.ps1 -OutDir . -Filename test.png
```

If this works, the worker will too.

---

## Dashboard

**Symptoms:** Can't load `http://<lan-ip>:8090/`.

### Port 8090 unreachable

```powershell
Test-NetConnection -Port 8090 -InformationLevel Quiet
```

If false: dashboard is not running.

```bash
python dashboard.py
```

### Dashboard 503s on `/api/vessel`

The bridge is down. Fix the bridge first (see above), then dashboard
will recover automatically.

### Captures list is empty

- The capture daemon has never run, OR
- `captures/v3/` doesn't exist yet, OR
- The day folder has zero `.png` files (very rare)

Check:

```bash
python capture_daemon.py status
ls captures/v3/
```

---

## Ollama

**Symptoms:** `ollama:11434 FAIL` in doctor. Local LLM not working.

### Connection refused

```powershell
ollama serve
```

(If ollama isn't installed: download from ollama.com.)

### Wrong port

Check `OLLAMA_HOST` env var. Default is `127.0.0.1:11434`. If you've
changed it, update `vessel.json` providers section.

### Model not pulled

```bash
ollama pull granite4.1:8b
```

The model is named in `vessel.json` `providers.ollama.model`.

---

## Vault

**Symptoms:** `vault:roundtrip FAIL`.

### Vault locked (cannot decrypt)

Two causes:

1. **Wrong Windows user.** The vault was created by a different user.
   Move vault.dat back to the original user, or re-create from scratch.
2. **Vault corrupted.** Restore from a backup of the same machine.

> ⚠️ Do not copy vault.dat between machines — DPAPI binds to
> user+machine. See `07_vault.md`.

### Vault tampered

AES-GCM authentication failed. The file has been modified outside
the vault API. Restore from backup, or `python vault.py init` to
start fresh (you'll lose all stored secrets).

---

## Capture Folder Won't Open

**Symptoms:** Tray → `Open today's captures` does nothing.

### Day folder doesn't exist

Today's capture hasn't happened yet. The folder is created on first
capture of the day. Wait, or trigger one:

```bash
python capture_v3.py --oneshot
```

### Permissions error

The Explorer session may not have read access. Verify:

```powershell
Get-Acl captures/v3/ | Format-List
```

Should show your user with Read/Write.

---

## Everything is On Fire

If multiple subsystems are broken at once:

1. **Don't panic.** The capture subsystem can be restored without
   affecting anything else.
2. **Check doctor first** to localize.
3. **Run `fix_priority0.bat`** if it's a bridge/dashboard issue.
4. **Run `python capture_daemon.py stop` then `run --auto`** if
   capture is broken.
5. **If still broken**, reboot the laptop. The system is designed to
   recover on reboot (assuming TZ Pro is launched afterward).

---

## Diagnostic One-Liners

```bash
# Is anything listening on the bridge port?
Test-NetConnection -Port 6006 -InformationLevel Quiet

# Is the bridge process alive?
Get-Process python | Where-Object { $_.CommandLine -like '*nmea_bridge*' }

# Latest capture on disk?
Get-ChildItem captures/v3/*/*.png | Sort-Object LastWriteTime -Descending | Select-Object -First 1

# Today's capture count?
(Get-ChildItem captures/v3/<today>/*.png).Count

# Stale NMEA heartbeat?
(Get-Item .last_nmea_heartbeat).LastWriteTime
```

---

## Getting Help

If you've tried the above and the system is still broken, gather:

1. `python doctor.py check --no-color > doctor.txt`
2. `Get-Process python | Format-List > processes.txt`
3. Last 50 lines of `logs/capture_v3.daemon-child.log`
4. Screenshot of TZ Pro chart (showing GPS or lack thereof)

…and attach to your support request.

## Related Docs

- All `engineer/*.md` — subsystem-specific deep dives
- `docs/TROUBLESHOOTING.md` — legacy troubleshooting (some entries still useful)
- `docs/HARDWARE_SETUP.md` — physical wiring reference