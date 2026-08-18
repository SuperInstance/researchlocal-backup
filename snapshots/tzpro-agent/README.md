# TZ Pro Agent

> The boat-agent platform. See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the
> full flagship vision, or jump to the current phase:
> [`docs/phases/phase-1.md`](docs/phases/phase-1.md).

## Quick start

```powershell
cd C:\Users\casey\tzpro-agent
python doctor.py check          # report health of all components
python doctor.py fix --yes      # auto-repair what's broken
```

## Phase 1 deliverables (in progress)

- Capture daemon that follows TZ Pro lifecycle
- Tray app with right-click menu (Open Dashboard, Status, etc.)
- LAN-reachable 3-panel dashboard
- Provider onboarding with secure key vault
- Per-cadence model preferences

See `docs/phases/phase-1.md` for details.

## Repository layout

```
tzpro-agent/
├── assets/              icon bundle (tray, shortcut, dashboard header)
├── captures/v3/         raw screenshot archive (organized by date+position)
├── docs/
│   ├── ROADMAP.md       full flagship vision
│   ├── BACKUP.md        backup procedure (keys excluded)
│   └── phases/          per-phase detail (phase-1 through phase-infinity)
├── scripts/             utility scripts (icon gen, backup, etc.)
├── capture_v3.py        screenshot capture daemon (10-min cadence)
├── capture_daemon.py    TZ Pro lifecycle supervisor (Phase 1)
├── tray_app.py          pystray front door (Phase 1)
├── dashboard.py         FastAPI LAN server (Phase 1)
├── dashboard/           dashboard frontend (HTML/JS/CSS)
├── providers/           provider abstraction (Phase 1)
├── vault.py             DPAPI key storage (Phase 1)
├── vessel.json          vessel + provider config (Phase 1)
├── doctor.py            health check + auto-repair
└── nmea_bridge.py       GPS serial → TCP/HTTP bridge
```
