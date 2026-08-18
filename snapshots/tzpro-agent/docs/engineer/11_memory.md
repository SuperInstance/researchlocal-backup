# Memory Subsystem — `memory/`

**Package:** `memory/`
**Priority:** Phase 2 (in development); foundation for the cascade.
**Owner:** tzpro-agent.

## What It Does

The agent's long-term memory. Three layers:

1. **Blobs** (`memory/blobs/`) — content-addressed storage for binary
   data (PNGs, audio clips, large payloads). Files are stored under
   their SHA-256 hash.
2. **Meta DB** (`memory/meta.db`) — SQLite index pointing at blobs
   plus structured queryable metadata.
3. **Manifests / exports / GC** — operational subdirs for retention
   and migration.

## Layout

```
memory/
├── blobs/                    # content-addressed storage
│   ├── ab/
│   │   └── abcd1234…         # first 2 hex chars as a shard dir
│   └── …
├── meta.db                   # SQLite (WAL mode: meta.db-wal, meta.db-shm)
├── manifests/                # per-day manifests for backup
├── exports/                  # ad-hoc exports (e.g. weekly summaries)
└── gc/                       # retention garbage collection state
```

## SQLite Schema (inferred)

The exact schema is owned by the cascade; check `memory/__init__.py`
and the `cascade/twin_sink.py` writer for the live schema. Likely
tables:

- `moments(id PRIMARY KEY, timestamp, source, importance, payload_json)`
- `blobs(hash PRIMARY KEY, size_bytes, first_seen_at, ref_count)`
- `tags(moment_id, tag)` (many-to-many)
- `correlations(id, ts_window_start, ts_window_end, members_json, score)`

## Content-Addressed Blobs

A blob is stored as `<2-char-prefix>/<full-hash>.<ext>`:

```python
import hashlib
from pathlib import Path

def store_blob(data: bytes, ext: str = "bin") -> str:
    h = hashlib.sha256(data).hexdigest()
    p = Path("memory/blobs") / h[:2] / f"{h}.{ext}"
    p.parent.mkdir(parents=True, exist_ok=True)
    if not p.exists():
        p.write_bytes(data)
    return h
```

Two writes of the same bytes produce the same path → deduplication
for free. Garbage collection drops blobs whose `ref_count == 0`.

## Putting a Moment into Memory

```python
# Pseudo-code (see cascade/twin_sink.py for the live version)
m = Moment(source="sounder", payload={...})
m_id = db.insert_moment(m)
if "screenshot_bytes" in m.payload:
    h = store_blob(m.payload["screenshot_bytes"], ext="png")
    db.link_blob(m_id, h, role="primary_image")
```

## Querying

For now, the dashboard and analyst use direct SQLite reads. Future
phases add a vector index (using `providers/embed.py`) for semantic
search.

## Retention

`cascade/retention.py` is the GC policy:

- Moments with `importance < 0.1` and `timestamp < now - 30d` are
  deleted.
- Blobs with `ref_count == 0` after GC are deleted.
- Cascade outputs older than 90 days are archived (compressed) and
  removed from `cascade_out/`.

## Export

```python
# Example: dump a day's Moments to a portable JSONL
python -m memory export --since 2026-07-23 --until 2026-07-24 \
    --out exports/2026-07-23.jsonl
```

The export format mirrors `cascade_out/records/*.jsonl`.

## Backup Interaction

`memory/` is in `.gitignore` — it is runtime data. See
`docs/BACKUP.md` for the policy:

- `meta.db` — backed up daily (small)
- `blobs/` — backed up weekly (large; can be reconstructed from
  `captures/v3/` if lost)
- `manifests/` — checked into git so we can verify a backup

## Status (as of `b64c2ef`)

- SQLite meta.db in place
- Blobs directory in use (check size)
- Manifests / exports / gc dirs scaffolded
- Cascade writes to memory via `twin_sink.py`

## Related Docs

- `cascade/README.md` — the cascade is the primary writer
- `docs/architecture/CAPTURE_PIPELINE.md` — what flows into memory
- `docs/architecture/PROJECTION_LAYER.md` — what reads from memory
- `docs/BACKUP.md` — backup policy