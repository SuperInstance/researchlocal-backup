# Daily Workflow

> **Audience:** the captain and any relief operator. Read this before
> your first day on the wheelhouse laptop.

## Morning Briefing (5 minutes)

1. **Power on the wheelhouse laptop.**
2. **Launch TZ Pro Professional** (`TimeZero.exe`). The boat icon
   appears in the center of the chart with no GPS — that's expected.
3. **Double-click the `TZ Pro Agent Tray` icon on the desktop.**
   The tray appears in the system tray (bottom-right, near the clock).
   Right-click → check `Status:` — should read `down/RUNNING/down`
   while TZ Pro is loading.
4. **Wait ~30 s for TZ Pro to fully load.** When the chart shows
   your boat at the correct position, capture has begun.
5. **Right-click tray → `Status:` should now read `trolling/RUNNING/RUNNING`.**
   Right-click → `Last capture: <Ns>` — should be under 600 s
   (next 10-min boundary).

> The whole flow is: TZ Pro → Tray icon → bridge starts → GPS shows
> on chart → capture flows. If any step doesn't work, see
> `13_troubleshooting.md`.

## During the Day

- **No interaction required.** The system runs in the background.
- **If the captain wants to inspect a specific capture:**
  Right-click tray → `Open latest capture` (or `Open today's captures`).
- **If something looks off:**
  Right-click tray → `Verify capture health`. Should print
  `VERDICT: HEALTHY`. Anything else, see `13_troubleshooting.md`.

## Evening Review (5 minutes)

1. **Right-click tray → `Open today's captures`.** Browse the folder.
   Each capture is a triplet of `.png` (echogram) + `.json` (metadata)
   + `.md` (human-readable).
2. **Open the dashboard in a browser** (`http://<lan-ip>:8090/`).
   - `/api/vessel` → JSON of current state (lat/lon/SOG/COG)
   - `/api/captures` → today's list
3. **Run the doctor once a day:**
   ```bash
   python doctor.py check
   ```
   Should print `9/9 healthy`. If less, see `13_troubleshooting.md`.

## Weekly Backup (15 minutes, Sunday)

1. Run `python scripts/cloud_backup.py` (see `docs/BACKUP.md`).
2. Verify the latest manifest is in `memory/manifests/`.
3. Optionally: copy `cascade_out/briefings/` to the cloud notebook.

## End of Trip / Power Down

The system is **stateless across power cycles** — it will resume
capture automatically when restarted. No special shutdown procedure.

If you want to be tidy:

```powershell
# From C:\Users\casey\tzpro-agent\
python capture_daemon.py stop
powershell -Command "Get-Process python -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like '*nmea_bridge*' } | Stop-Process -Force"
```

(Or just shut down Windows — the daemon's signals are graceful.)

## Common Operator Misconceptions

| Misconception | Reality |
|---|---|
| "TZ Pro is showing my position, so capture is working." | Capture requires the **daemon** to be running, not just TZ Pro. Check tray status. |
| "If the boat icon on TZ Pro moves, capture is happening." | The bridge relays GPS but capture is a separate subsystem. Both must work. |
| "More captures is better." | The 10-minute cadence is intentional — over-capturing wastes disk and floods analyzers. |
| "I should restart everything if something is weird." | Often one subsystem is broken; check doctor first. |

## Related Docs

- `docs/BOAT_RUNBOOK.md` — original operator runbook (still valid)
- `docs/DAILY_WORKFLOW.md` — same content, doc-consolidation target
- `docs/HARDWARE_SETUP.md` — physical wiring reference
- `docs/engineer/13_troubleshooting.md` — symptom → fix cookbook
- `docs/engineer/04_tray_app.md` — what each tray menu item does