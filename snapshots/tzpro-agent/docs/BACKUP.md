# Backup & Restore Procedure

## What gets backed up

| Path | Contents | Why |
|---|---|---|
| `tzpro-agent-data/` | All moments, captures, analyses, embeddings | Primary state |
| `tzpro-agent/vessel.json` | Provider config (model preferences, base URLs) | Routing config |
| `tzpro-agent/db/` | Local SQLite databases | Indexed state |
| `tzpro-agent/doctor.py`, `dashboard.py`, `tray_app.py`, `capture_daemon.py`, etc. | Application code | Reproducible build |

## What does NOT get backed up

| Path | Why excluded |
|---|---|
| `~/.tzpro-agent/vault.dat` | Encrypted API keys. **Restoring secrets across machines is a security risk** (different machine keys, different threat model). User re-enters keys on restore. |
| `~/.tzpro-agent/sessions/` (cookies/tokens) | Active session state is ephemeral; rebuilding it is trivial |
| `__pycache__/`, `.pyc` files | Regenerable from source |

## Why keys aren't backed up

1. **Cross-machine decryption fails.** Windows DPAPI encrypts with the
   current user's credentials. A backup restored on a different machine
   (or even a different user account on the same machine) cannot decrypt
   it. So the key would be effectively lost.
2. **Backup media is a larger attack surface.** USB drives, NAS shares,
   cloud backups — they're often less protected than the local disk.
   Keeping secrets out of them is defense in depth.
3. **Captain's own account vs fleet accounts.** If a captain shares a
   backup with someone else for debugging, they shouldn't accidentally
   leak their API keys. Excluding the vault makes this safe by default.
4. **Restoration is an explicit, attended operation.** If the user is
   sitting at a machine actively restoring a backup, they can re-enter
   their 4-6 API keys in 2 minutes. That's a small cost for not having
   those keys persist in backup media for years.

## The exclusion rule, in script form

```python
# backup.py
EXCLUDE_PATTERNS = [
    "**/vault.dat",
    "**/vault.dat.bak",
    "**/.DS_Store",
    "**/__pycache__/**",
    "**/*.pyc",
]
```

When the backup tool encounters any path matching these patterns, it
skips the file **silently** and continues. There's no "include anyway"
flag — keys never ride along.

## Verification

After any backup, run:

```powershell
python -c "
from pathlib import Path
import re
backup_root = Path(r'D:\backups\tzpro-agent')
patterns = ['**/vault.dat', '**/vault.dat.bak']
hits = []
for pat in patterns:
    hits.extend(backup_root.glob(pat))
if hits:
    print(f'WARNING: {len(hits)} vault file(s) found in backup!')
    for h in hits:
        print(f'  {h}')
else:
    print('OK: no vault files in backup')
"
```

This script lives in `scripts/verify_backup_safety.py`.

## Restore procedure

1. Restore the backup to the new machine
2. Install TZ Pro Agent code from the same release tag
3. Launch the dashboard — it will detect an empty vault
4. Re-enter API keys via the onboarding form
5. Test each provider with the "Save & Test" button
6. Run `python doctor.py check` to confirm all green
