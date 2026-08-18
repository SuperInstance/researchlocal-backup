# Upstream Merge Audit — SuperInstance/hermes-construct

**Date:** 2026-06-01  
**Fork:** SuperInstance/hermes-construct ← NousResearch/hermes-agent  
**Status:** 38 commits behind, 11 commits ahead

---

## Summary

| Category | Count | Priority |
|----------|-------|----------|
| Security fixes | 2 | 🔴 Critical — merge immediately |
| Bug fixes | 12 | 🟡 Important — merge before next release |
| Feature additions | 14 | 🔵 Evaluate individually |
| Minor/chore | 10 | ⚪ Low priority |

**Merge conflict:** 1 file (`.env.example`) — trivial, manual resolution in 30 seconds.  
**Total upstream changes:** 608 files, +128K / -5K lines.

---

## 🔴 Critical Security Fixes (merge NOW)

| Commit | Description | Conflicts? |
|--------|-------------|------------|
| `4e9d886` | Fix approval: pair terminal-side gate for ~/.hermes/config.yaml writes | None |
| `8f2931e` | Fix file_tools: block agent writes to ~/.hermes/config.yaml — prevent silent approval bypass | None |

These prevent an agent from silently modifying its own config to bypass approval gates. **Merge these two commits immediately**, even if we defer the rest.

## 🟡 Important Bug Fixes

| Commit | Description | Conflicts? |
|--------|-------------|------------|
| `b964627` | Guard os.fchmod for Windows in atomic_json_write | None |
| `023149f` | Stop reporting broken streams as output-length truncation | None |
| `ba6ffd4` | Stop flagging benign skill content + honor skill ignore files | None |
| `6c73e8f` | Keep code blocks verbatim in cleaned text when media present | None |
| `3ccf4fd` | Skip MEDIA: tags inside code blocks and blockquotes | None |
| `521d069` | Restrict auto-appended media to producer tools | None |
| `fb1b681` | Keep JSON-embedded MEDIA: text verbatim in cleaned output | None |
| `e8827ef` | Skip MEDIA: inside serialized JSON string values | None |
| `b3aaf26` | Discover Playwright headless_shell browser in Docker | None |
| `f106e58` | Create s6 envdir before browser path export in Docker | None |
| `bdceedf` | Fix Docker: chown hermes-owned top-level state files on boot | None |
| `b14e15c` | Clean service restart notifications | None |

## 🔵 Feature Additions (evaluate)

| Commit | Description | Notes |
|--------|-------------|-------|
| `b571ec2` | Full admin dashboard — MCP, pairing, webhooks, credentials, memory, gateway, ops | Major UI addition. Review for compatibility with our ensign configs. |
| `2ed9637` | Blank-slate skills — install --no-skills + opt-out/opt-in | Aligns with our module system vision. Good to merge. |
| `70e1571` | Prune built-in skills after inactivity + track usage | Useful for our self-configuring agent. |
| `0622a70` | /undo [N] on messaging platforms | Nice UX feature. |
| `3e59be0` | Messages.active flag + rewind primitives | New state management. Review API changes. |
| `3e59be0` | /rewind through command.dispatch + prefill payload | Related to undo/rewind. |
| `92a567d` | Explain Quick Setup vs Full setup in first-time menu | Aligns with our onboarding goals. |
| `e1eba6f` | Fix dashboard-auth: drop /api/* from OAuth next= round trip | Dashboard fix. |
| `7fbe9b7` | Add PATCH /api/sessions/{id} for rename | Dashboard fix. |
| `c1a531d` | Guard update endpoint in Docker with structured guidance | Dashboard fix. |
| `e3b3d4d` | Drop files anywhere in chat area (desktop) | Desktop UX. |
| `380ce47` | Remove privileges drop when never ran as root | Docker fix. |
| `a60bff2` | /usr/bin/tini compatibility shim for legacy wrappers | Docker fix. |
| `740fb28` | Chown ensure_hermes_home dirs to HERMES_UID/GID in Docker | Docker fix. |
| `e3b3d4d` | MiniMax-M3 native provider + 1M context | New model support. |
| `79e7e7a` | Make locally-built macOS app relaunchable after self-update | Desktop fix. |

## ⚪ Minor/Chore

| Commit | Description |
|--------|-------------|
| `a5371b3` | Add benfrank241 to AUTHOR_MAP |
| `ef3a650` | Map Subway2023 for PR salvage |
| `92a567d` | Regen model catalog + fix GUI test macos-fixup |
| `ec6261a` | Map VinciZhu to AUTHOR_MAP |
| `e3b3d4d` | Map polnikale for PR attribution |
| `0bc616e` | Darken light-mode code comment color |
| `064875a` | Docker s6 /init support |
| `740fb28` | Docker config chown |
| `c1a531d` | Dashboard update guard |
| Various | Attribution mapping, CI fixes |

---

## Our Commits (11 ahead)

| Commit | Description | Merge Risk |
|--------|-------------|------------|
| `657add7` | Gitignore dist/ deployment bundles | None |
| `523e56d` | Oracle ARM deployment scripts | None |
| `3543b6f` | Cross-compile to aarch64 | None |
| `39e7572` | Replace blocking_lock() with AtomicBool | None |
| `c872f5f` | Ensign lifecycle, graceful degradation, provenance | None |
| `70a9b33` | Examples, templates, quickstart, architecture docs | None |
| `fb7c6f8` | Solutions for three hardest integration puzzles | None |
| `fa118ed` | Rust binary v0.1 prototype | None |
| `ba81782` | v0.1 prototype spec | None |
| `073a359` | SCHEMAS.md — kernel type definitions | None |
| `31db329` | PLATO Build Plan — 7-phase refactoring | None |

All our additions are in new files or isolated sections. Very low merge risk.

---

## Recommended Merge Strategy

### Phase 1: Security (do now)
```bash
git cherry-pick 4e9d886 8f2931e
```

### Phase 2: Bug fixes (this week)
```bash
git cherry-pick b964627 023149f ba6ffd4 6c73e8f 3ccf4fd 521d069 fb1b681 e8827ef b3aaf26 f106e58 bdceedf b14e15c
```

### Phase 3: Full merge (after testing)
```bash
git merge upstream/main
# Resolve .env.example conflict manually
# Test: cargo build --release --target aarch64-unknown-linux-gnu
# Test: python cli.py --help
```

### Phase 4: Integrate features we want
- Dashboard (b571ec2) — review after we decide on UI direction
- Blank-slate skills (2ed9637) — aligns with module system
- Skill pruning (70e1571) — useful for self-configuring agent
- Undo/rewind (0622a70, 3e59be0) — good UX

---

## Conflict Resolution

Only `.env.example` conflicts. Both sides added API key placeholders. Resolution:

```bash
# Keep our ensign + deployment keys, add upstream's new keys
git checkout --ours .env.example
# Then manually add any new keys from upstream that we want
```

---

*Generated 2026-06-01. Re-run after merging upstream to stay current.*
