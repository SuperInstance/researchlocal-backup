# Phase 8 — Cloudflare Sync

> **Status:** Planned — vision-mode
> **Depends on:** Phase 1-7 mature on at least one boat

## Goal

The same data, the same dashboard, accessible from anywhere. Multi-device
native. Fleet-scaling nearly automatic when ready.

## Architecture

| Local (already exists) | Cloudflare (new) |
|---|---|
| SQLite | D1 |
| Filesystem (PNGs, audio) | R2 |
| Vector index | Vectorize |
| Scratch state | KV |
| Per-vessel state | Durable Objects |
| Analysis queue | Queues |
| Provider calls | Workers AI (default) |

The dashboard frontend doesn't change. The API layer talks to either
local or cloud; a session preference decides which.

## Captain's own Cloudflare account

A captain's entire system can run on their own Cloudflare account. The
free tier covers a single-boat workload easily:

- 100k requests/day free
- R2: 10 GB storage free, 10M reads free, 1M writes free
- D1: 5 GB storage, 5B reads/month, 50M writes/day
- Vectorize: 30M queried vectors free
- Workers AI: 10k neurons/day free

If the captain exceeds free tier (rare for one boat), they can configure
a fallback provider (DeepInfra, OpenRouter) for general-purpose AI work
without changing the architecture.

## Fleet scaling

Once multiple boats sync to the same Cloudflare account (or a shared
fleet account), we get fleet intelligence essentially for free:

- Aggregate catch rates by region, by tide, by depth
- Cross-vessel pattern detection (everyone saw the same bait layer
  yesterday because it was there)
- Shared anonymized training data for the local models

## What's new in Phase 8

- `sync/` package — local→cloud replicator
- Cloudflare Worker dashboard (mirror of local FastAPI)
- Auth layer (Cloudflare Access or similar) — single boat doesn't need
  it, fleet does
- Fleet aggregation queries
