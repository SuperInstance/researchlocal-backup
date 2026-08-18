# Vault — `vault.py`

**File:** `vault.py` (~300 lines)
**Priority:** Security boundary — all API keys live here.
**Owner:** tzpro-agent.
**Path:** `~/.tzpro-agent/vault.dat` (NOT in the repo, NOT in backups).

## What It Does

Encrypted secret storage for API keys and provider credentials. Backs
every provider that needs an API key (DeepInfra, OpenAI, OpenRouter,
Grok, DeepSeek, Z.AI, etc.).

Design constraints:

- **At-rest encryption:** nothing useful on disk without the current
  Windows user.
- **Backup-safe:** the vault path is on the exclusion list in
  `docs/BACKUP.md`. Even if the file leaks, it's encrypted to the
  current user.
- **Portable enough:** the on-disk format is a self-contained JSON
  object that could be migrated to a different store later by writing
  a sibling backend.

## Encryption Chain

```
1. DPAPI (Windows Data Protection API) protects a per-user secret key
   (32 random bytes). DPAPI ties decryption to the current Windows user
   — another user on the same machine cannot decrypt.

2. That secret key + a per-vault random salt derive an AES-256 key
   via HKDF-SHA256.

3. The inner JSON (all secrets) is encrypted with AES-GCM
   (authenticated — detects tampering).

4. The DPAPI-protected secret key is stored alongside and rotated on
   every save.
```

> **Mac/Linux fallback:** If `win32crypt` is unavailable, the vault
> falls back to a file-mode-only key (still encrypted at rest, just
> portable to other machines). For the production deployment
> (Windows wheelhouse laptop) DPAPI is always available.

## On-Disk Format (`~/.tzpro-agent/vault.dat`)

```json
{
  "version": 1,
  "kdf_salt": "<base64 16 bytes>",
  "blob": "<base64 AES-GCM ciphertext of inner JSON>",
  "created_at": "ISO8601",
  "updated_at": "ISO8601"
}
```

The inner JSON (post-decryption) is a flat dict:

```json
{
  "deepinfra": {"api_key": "..."},
  "openai":    {"api_key": "..."},
  "openrouter":{"api_key": "..."}
}
```

## API

```python
from vault import Vault

vault = Vault()                            # default path
vault.set("deepinfra", {"api_key": "..."}) # write/update a secret
vault.get("deepinfra")                      # read (decrypted)
vault.list_names()                          # ["deepinfra", "openai", ...]
vault.delete("deepinfra")                   # remove a secret
vault.has("deepinfra")                      # bool
vault.test("deepinfra")                     # round-trip check
```

## CLI

```bash
python vault.py set <name> <json>
python vault.py get <name>
python vault.py list
python vault.py delete <name>
python vault.py test <name>
python vault.py init                       # create empty vault
```

`_cli(argv)` (`vault.py`) is a thin wrapper that prints results.

## Error Types

| Exception | Meaning |
|---|---|
| `VaultError` | base class |
| `VaultLockedError` | cannot decrypt (wrong Windows user, file corrupted) |
| `VaultTamperedError` | AES-GCM authentication failed |
| `VaultNotFoundError` | the named secret doesn't exist |

The doctor check `vault:roundtrip` decrypts a known marker to verify
the vault is usable.

## Bootstrap Sequence

```bash
# First-time setup on a new machine
python vault.py init
python vault.py set deepinfra '{"api_key": "<paste>"}'
python vault.py test deepinfra
```

## Cross-User / Cross-Machine Caveat

> ⚠️ **DO NOT** copy `vault.dat` to a different Windows user or
> different machine. DPAPI binds the key to user+machine; the file
> will be unreadable. To migrate:
>
> 1. On the old machine: `python vault.py list` (read names only)
> 2. Manually re-enter each secret on the new machine:
>    `python vault.py set <name> <json>`
>
> This is intentional. It's the suit-vs-person privacy boundary.

## Verified Line Numbers (as of `b64c2ef`)

- Default path: `vault.py:69-72`
- Error classes: `vault.py:79-…`
- DPAPI helpers: `vault.py` mid-file (search `win32crypt`)
- `Vault` class: `vault.py` mid-file (search `class Vault`)
- CLI: search `def _cli`

## Related Docs

- `docs/BACKUP.md` — vault is on the exclusion list
- `docs/SUIT_VS_PERSON.md` — privacy charter (vault is the gate)
- `docs/engineer/08_providers.md` — who consumes the vault