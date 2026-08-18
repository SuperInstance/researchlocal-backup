"""
vault.py - Encrypted secret storage for API keys and provider credentials.

Design goals:
  - At-rest encryption: nothing useful on disk without the current
    Windows user.
  - Backup-safe: this file's path (vault.dat) is in BACKUP.md's
    exclusion list. Even if it leaks, the data is encrypted to the
    current user.
  - Portable enough: the encrypted blob is a self-contained JSON
    object that could be migrated to a different secret store later
    (e.g. macOS Keychain on a Mac build) by writing a sibling vault
    backend.

Schema of the on-disk file (vault.dat):

    {
      "version": 1,
      "kdf_salt": "<base64>",          # 16 random bytes for HKDF
      "blob": "<base64>",              # AES-GCM ciphertext of the inner JSON
      "created_at": "ISO8601",
      "updated_at": "ISO8601",
    }

Encryption chain:
  1. DPAPI protects a per-user secret key (32 random bytes).
  2. That secret key + a random salt derive an AES-256 key via HKDF-SHA256.
  3. The inner JSON (all secrets) is encrypted with AES-GCM (authenticated).
  4. The DPAPI-protected secret key is stored alongside (rotated per save).

API:
  vault = Vault()                  # default path ~/.tzpro-agent/vault.dat
  vault.set("deepinfra", {...})    # write/update a secret
  vault.get("deepinfra")           # read (decrypted)
  vault.list_names()               # ["deepinfra", "openai", ...]  (no values)
  vault.delete("deepinfra")        # remove a secret
  vault.has("deepinfra")           # bool
  vault.test("deepinfra")          # round-trip check (raises on tamper)
"""

from __future__ import annotations

import base64
import json
import os
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional

from cryptography.hazmat.primitives.ciphers.aead import AESGCM
from cryptography.hazmat.primitives.kdf.hkdf import HKDF
from cryptography.hazmat.primitives import hashes

# DPAPI comes from pywin32 on Windows; on other platforms we fall back
# to a file-mode-only key (still encrypted at rest, just portable).
if sys.platform == "win32":
    import win32crypt  # type: ignore
    _HAS_DPAPI = True
else:  # pragma: no cover - non-Windows dev only
    _HAS_DPAPI = False


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

def _default_vault_path() -> Path:
    """Default location: ~/.tzpro-agent/vault.dat (excluded from backups)."""
    home = Path(os.path.expanduser("~"))
    return home / ".tzpro-agent" / "vault.dat"


# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------

class VaultError(Exception):
    """Base class for vault errors."""


class VaultLockedError(VaultError):
    """Vault exists but cannot be decrypted with current user credentials."""


class VaultTamperedError(VaultError):
    """Vault file was modified outside this module (AES-GCM auth tag failed)."""


class VaultNotFoundError(VaultError):
    """Requested secret key is not present."""


# ---------------------------------------------------------------------------
# Key management
# ---------------------------------------------------------------------------

def _new_secret_key() -> bytes:
    """32 random bytes, encrypted via DPAPI for the current user."""
    raw = os.urandom(32)
    if _HAS_DPAPI:
        # DPAPI encrypts for the current user; only that user (on this
        # machine) can decrypt it. No flags = current user scope.
        description = "tzpro-agent/vault:master-key:v1"
        return win32crypt.CryptProtectData(raw, description, None, None, None, 0)
    # Non-Windows fallback (e.g. for testing on Linux). Less secure
    # but still authenticated at rest via AES-GCM.
    return raw  # type: ignore[return-value]


def _unprotect_secret_key(protected: bytes) -> bytes:
    """Reverse of _new_secret_key()."""
    if _HAS_DPAPI:
        return win32crypt.CryptUnprotectData(protected, None, None, None, 0)[1]
    return protected  # type: ignore[return-value]


def _derive_aes_key(secret_key: bytes, salt: bytes) -> bytes:
    """HKDF-SHA256 -> 32-byte AES key."""
    return HKDF(
        algorithm=hashes.SHA256(),
        length=32,
        salt=salt,
        info=b"tzpro-agent/vault/aes",
    ).derive(secret_key)


# ---------------------------------------------------------------------------
# Vault
# ---------------------------------------------------------------------------

@dataclass
class Vault:
    path: Path = None  # type: ignore[assignment]

    def __post_init__(self) -> None:
        if self.path is None:
            self.path = _default_vault_path()
        self.path = Path(self.path)
        self.path.parent.mkdir(parents=True, exist_ok=True)

    # ---- low-level I/O --------------------------------------------------

    def _read_header(self) -> Optional[dict]:
        if not self.path.exists():
            return None
        try:
            return json.loads(self.path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise VaultError(f"vault file unreadable: {exc}") from exc

    def _write_header(self, header: dict) -> None:
        # Atomic write: write to .tmp, then rename.
        tmp = self.path.with_suffix(self.path.suffix + ".tmp")
        tmp.write_text(json.dumps(header, indent=2), encoding="utf-8")
        os.replace(tmp, self.path)
        # Best-effort: lock down permissions on Windows.
        try:
            import stat
            os.chmod(self.path, stat.S_IRUSR | stat.S_IWUSR)
        except OSError:
            pass

    def _decrypt(self, header: dict) -> dict:
        try:
            protected = base64.b64decode(header["secret_key_b64"])
            salt = base64.b64decode(header["kdf_salt"])
            blob = base64.b64decode(header["blob"])
        except KeyError as exc:
            raise VaultError(f"vault header malformed: missing {exc}") from exc

        try:
            secret_key = _unprotect_secret_key(protected)
        except Exception as exc:  # pragma: no cover - DPAPI failure path
            raise VaultLockedError(
                "cannot decrypt vault: not current user or different machine"
            ) from exc

        aes_key = _derive_aes_key(secret_key, salt)
        aes = AESGCM(aes_key)
        # Layout: nonce (12 bytes) || ciphertext || tag (16 bytes appended).
        # cryptography's AESGCM expects ciphertext+tag concatenated in `data`.
        try:
            nonce = blob[:12]
            data = blob[12:]
            plaintext = aes.decrypt(nonce, data, None)
        except Exception as exc:
            raise VaultTamperedError(
                "vault file failed authentication - it was modified outside vault.py"
            ) from exc

        try:
            return json.loads(plaintext.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise VaultError(f"vault inner payload unreadable: {exc}") from exc

    def _encrypt(self, inner: dict) -> dict:
        salt = os.urandom(16)
        # Round-trip through DPAPI so we hold the raw 32-byte key for HKDF.
        # The protected form is what gets persisted on disk; only the raw
        # bytes can be used as KDF input.
        protected_key = _new_secret_key()
        secret_key = _unprotect_secret_key(protected_key)
        aes_key = _derive_aes_key(secret_key, salt)
        aes = AESGCM(aes_key)
        nonce = os.urandom(12)
        plaintext = json.dumps(inner, separators=(",", ":")).encode("utf-8")
        ciphertext = aes.encrypt(nonce, plaintext, None)
        blob = nonce + ciphertext

        now = datetime.now(timezone.utc).isoformat()
        return {
            "version": 1,
            "kdf_salt": base64.b64encode(salt).decode("ascii"),
            # Persist the DPAPI-protected key so only the current user can
            # decrypt the vault on this machine.
            "secret_key_b64": base64.b64encode(protected_key).decode("ascii"),
            "blob": base64.b64encode(blob).decode("ascii"),
            "created_at": now,
            "updated_at": now,
        }

    # ---- public API -----------------------------------------------------

    def _load(self) -> dict:
        """Return the inner dict (all secrets), creating empty if needed."""
        header = self._read_header()
        if header is None:
            return {}
        return self._decrypt(header)

    def _save(self, inner: dict) -> None:
        new_header = self._encrypt(inner)
        # Preserve created_at from existing file if present.
        existing = self._read_header()
        if existing and "created_at" in existing:
            new_header["created_at"] = existing["created_at"]
        self._write_header(new_header)

    def set(self, name: str, secret: dict) -> None:
        """Write or replace a named secret. `secret` must be JSON-serializable."""
        if not isinstance(secret, dict):
            raise VaultError(f"secret for {name!r} must be a dict, got {type(secret).__name__}")
        inner = self._load()
        inner[name] = {
            "value": secret,
            "updated_at": datetime.now(timezone.utc).isoformat(),
        }
        self._save(inner)

    def get(self, name: str) -> dict:
        """Return the secret dict for `name`. Raises VaultNotFoundError if absent."""
        inner = self._load()
        if name not in inner:
            raise VaultNotFoundError(f"no secret named {name!r}")
        return inner[name]["value"]

    def try_get(self, name: str) -> Optional[dict]:
        """Like get() but returns None instead of raising on absence."""
        try:
            return self.get(name)
        except VaultNotFoundError:
            return None

    def has(self, name: str) -> bool:
        return name in self._load()

    def delete(self, name: str) -> bool:
        """Remove a secret. Returns True if removed, False if it wasn't there."""
        inner = self._load()
        if name not in inner:
            return False
        del inner[name]
        self._save(inner)
        return True

    def list_names(self) -> list[str]:
        """Return sorted list of stored secret names (no values)."""
        return sorted(self._load().keys())

    def list_summaries(self) -> list[dict]:
        """Return [{name, updated_at}] for UI display. No secret values."""
        inner = self._load()
        return [
            {"name": name, "updated_at": data.get("updated_at", "")}
            for name, data in sorted(inner.items())
        ]

    def test(self, name: str) -> tuple[bool, str]:
        """Round-trip self-check. Returns (ok, message)."""
        try:
            value = self.get(name)
            return True, f"ok: secret {name!r} reads back ({len(json.dumps(value))} bytes)"
        except VaultNotFoundError:
            return False, f"not stored: {name!r}"
        except (VaultTamperedError, VaultLockedError) as exc:
            return False, f"vault integrity: {exc}"
        except Exception as exc:
            return False, f"unexpected: {exc!r}"

    def test_all(self) -> list[dict]:
        """Run test() on every stored secret. Returns [{name, ok, message}]."""
        return [
            {"name": name, **dict(zip(("ok", "message"), self.test(name)))}
            for name in self.list_names()
        ]


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _cli(argv: list[str]) -> int:
    import argparse
    parser = argparse.ArgumentParser(description="vault utility for tzpro-agent")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_list = sub.add_parser("list", help="list stored secret names")
    p_list.add_argument("--values", action="store_true",
                        help="also print values (UNSAFE; for debugging only)")

    p_set = sub.add_parser("set", help="set a secret from JSON on stdin")
    p_set.add_argument("name")

    p_get = sub.add_parser("get", help="print a secret as JSON")
    p_get.add_argument("name")

    p_del = sub.add_parser("delete", help="remove a secret")
    p_del.add_argument("name")

    p_test = sub.add_parser("test", help="round-trip check on one or all secrets")
    p_test.add_argument("name", nargs="?")

    p_path = sub.add_parser("path", help="print vault file path")

    args = parser.parse_args(argv)
    v = Vault()

    if args.cmd == "path":
        print(v.path)
        return 0
    if args.cmd == "list":
        if args.values:
            for name in v.list_names():
                print(f"{name} = {json.dumps(v.get(name))}")
        else:
            for s in v.list_summaries():
                print(f"{s['name']:30}  {s['updated_at']}")
        return 0
    if args.cmd == "set":
        raw = sys.stdin.read().strip()
        try:
            value = json.loads(raw)
        except json.JSONDecodeError as exc:
            print(f"invalid JSON on stdin: {exc}", file=sys.stderr)
            return 2
        v.set(args.name, value)
        print(f"stored {args.name!r}")
        return 0
    if args.cmd == "get":
        try:
            print(json.dumps(v.get(args.name), indent=2))
        except VaultNotFoundError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 1
        return 0
    if args.cmd == "delete":
        removed = v.delete(args.name)
        print("removed" if removed else "not present")
        return 0
    if args.cmd == "test":
        if args.name:
            ok, msg = v.test(args.name)
            print(("OK  " if ok else "FAIL ") + msg)
            return 0 if ok else 1
        for r in v.test_all():
            mark = "OK  " if r["ok"] else "FAIL"
            print(f"[{mark}] {r['name']:30}  {r['message']}")
        return 0
    parser.error("unknown command")
    return 2  # unreachable


if __name__ == "__main__":
    sys.exit(_cli(sys.argv[1:]))
