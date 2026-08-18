# Session Boot — The 5-Minute Cold-Boot Ritual

> **When to use this:** You just opened the laptop. You have never seen
> this system before (or your memory of it is gone — reboot, collapse,
> fresh clone). You need to know *what is true right now* before you
> touch anything.

---

## The ritual (do this in order, every time)

### Step 0 — Orient (60 seconds)

1. **Confirm where you are:** `pwd` should resolve to `tzpro-agent/`.
   If not, `cd C:\Users\casey\tzpro-agent`.
2. **Confirm TZ Pro is the captain's active app.** Check the system
   tray. Look at the wheelhouse. The captain will tell you if you ask.
3. **Confirm your identity.** You are an agent operating this system
   on behalf of the captain (Casey). Your job is to keep the capture
   pipeline durable, the GPS visible to TZ Pro, and the data flowing
   to disk.

### Step 1 — Doctor (30 seconds)

```powershell
python doctor.py check
```

**Expected output:** `9/9 healthy`. If you see anything else, **stop**
and run `python doctor.py fix --yes` (which auto-repairs the bridge,
dashboard, and capture daemon). Then re-run `check`. If still failing,
go to `docs/engineer/13_troubleshooting.md`.

### Step 2 — Capture health (15 seconds)

```powershell
python capture_daemon.py verify
```

**Expected output:** `VERDICT: HEALTHY` with `Latest capture: <150s
ago`. If `STALE` or `UNHEALTHY`, check:
- Is TZ Pro actually running? (`Get-Process TimeZero`)
- Is the tray icon present in the system tray? (If not, run
  `python tray_app.py` to launch it; the tray auto-starts the daemon.)
- Are there errors in `logs/capture_daemon.log`?

### Step 3 — GPS visibility (15 seconds)

```powershell
Test-NetConnection -Port 6006 -InformationLevel Quiet
```

**Expected:** `True`. If `False`, the bridge is down. Run
`python nmea_bridge.py --port COM6 --baud 4800` (use `--diag` for a
30-second diagnostic that shows parsed sentences).

### Step 4 — Memory state (30 seconds)

```powershell
python -c "import db; c=db.connect(); print('moments:', c.execute('SELECT COUNT(*) FROM moments').fetchone()[0])"
```

**Expected:** A growing number. If it is `0`, the capture pipeline has
never run on this machine — that is fine for a fresh install but a red
flag for a returning session. Check `captures/v3/` for raw files.

### Step 5 — Git state (15 seconds)

```powershell
git status --short
git log --oneline -5
```

**Expected:** Clean working tree, head pointing at a recent commit
(within a few days). If there are uncommitted changes, **read them**
before doing anything else — someone (you, last session) was mid-thought.

---

## Time budget

The whole ritual should take under **2 minutes** on a healthy system.
If it takes longer, you have discovered an issue. Document what you
found in `docs/PLANS/daily/YYYY-MM-DD.md` with a SESSION-NOTE before
proceeding.

---

## What "healthy" means

The system is **healthy** when:
- Doctor reports `9/9 healthy`.
- Capture verify reports `HEALTHY` and the latest capture is < 15
  minutes old.
- TZ Pro is receiving GPS fixes (`bridge:serial` reports `fix_q=1`
  and `sats >= 4`).
- TZ Pro is connected to the bridge on TCP:6006
  (`bridge:http:/health` reports `tcp_clients >= 1` when TZ Pro is
  running, `0` when it isn't — both are valid).

The system is **operating** but **not delivering value** when:
- Doctor is 9/9 but capture is stale (TZ Pro is off, or the daemon
  hasn't noticed yet — the 30-second grace window).
- All systems report OK but `captures/v3/` has no captures newer than
  yesterday (TZ Pro hasn't been on the wheelhouse today).

The system is **broken** when:
- Doctor reports anything < 9/9 and `doctor.py fix --yes` does not
  restore it.
- Capture verify reports `UNHEALTHY` or `DEGRADED`.
- TZ Pro cannot see GPS position (visible in TZ Pro's data window).

---

## The "I just want to know what to do" version

```powershell
cd C:\Users\casey\tzpro-agent
python doctor.py check
python capture_daemon.py verify
```

Two commands. 45 seconds. If both pass, **you are operational**. Now go
read `02_state_of_the_system.md` to understand what you are responsible
for, then `03_decisions_log.md` before proposing changes.
