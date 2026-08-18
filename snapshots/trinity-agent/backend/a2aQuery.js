/**
 * backend/a2aQuery.js
 * ----------------------------------------------------------------------------
 * The read-side query layer for the A2A action log.
 *
 * Where A2aLog is a write-side appender (writes JSONL records, single-line
 * per record, rotation by size), A2aQuery is a read-side analyst that
 * streams the JSONL files and answers questions like:
 *
 *   - "Show me every action of kind=morph_to_hazard_mode in the last hour."
 *   - "How many A2AAction records fired per minute over the last 24 hours?"
 *   - "What's the top 5 most-common reasons given for raise_alert?"
 *   - "What's the priority distribution for all actions emitted yesterday?"
 *
 * Design constraints:
 *   - Pure JS. No new dependencies. JSONL is already self-describing.
 *   - Streaming reads. Files can grow to many MB; we never load them all.
 *   - Filtering happens during the read, so a 100MB log with 100 matching
 *     records returns 100 records (not 100MB + parse cost).
 *   - Doesn't mutate the log. Read-only fs access.
 *   - Tolerates corrupt lines (skips, warns in verbose mode).
 *
 * Why no DuckDB?
 *   DuckDB would be a 50MB native dep with platform-specific binaries.
 *   For our scale (thousands of actions per voyage day, not millions),
 *   a streaming JS filter is fast enough and stays in-repo with no
 *   native build step. If we ever cross 100K records/day or need SQL,
 *   we revisit.
 *
 * Construction:
 *   const q = new A2aQuery({ dir: "./logs/a2a" });
 *   const hazards = await q.query({ action: "morph_to_hazard_mode",
 *                                   since: "2026-07-25T00:00:00Z" });
 *
 *   const counts = await q.topActions(5);
 *   // => [ { action: "morph_to_hazard_mode", count: 12 }, ... ]
 *
 *   const buckets = await q.bucketBy({ intervalMs: 60_000, since: ... });
 *   // => [ { ts: "2026-07-25T22:00:00Z", count: 3 }, ... ]
 * ----------------------------------------------------------------------------
 */

"use strict";

const fs   = require("fs");
const path = require("path");
const { promisify } = require("util");

const fsReaddir = promisify(fs.readdir);
const fsStat    = promisify(fs.stat);

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const DEFAULT_DIR = "./logs/a2a";
const NAME_PREFIX = "a2a"; // matches A2aLog's default

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * True iff the filename looks like an A2A log file (a2a-*.jsonl).
 * Tolerant: doesn't require exact timestamp format, just prefix+ext.
 */
function isA2aLogFilename(name) {
  if (typeof name !== "string") return false;
  if (!name.startsWith(NAME_PREFIX)) return false;
  if (!name.endsWith(".jsonl")) return false;
  return true;
}

/**
 * Parse an ISO timestamp into a millisecond epoch. Returns null on garbage.
 */
function parseIsoMs(s) {
  if (typeof s !== "string") return null;
  const t = Date.parse(s);
  return Number.isFinite(t) ? t : null;
}

/**
 * Build a getter for a dot-notation field path. One level of nesting only.
 * "priority" -> rec.priority; "payload.depth" -> rec.payload?.depth.
 */
function makeFieldGetter(field) {
  if (field.indexOf(".") === -1) return (rec) => rec == null ? undefined : rec[field];
  const [head, tail] = field.split(".", 2);
  return (rec) => {
    if (rec == null) return undefined;
    const inner = rec[head];
    if (inner == null) return undefined;
    return inner[tail];
  };
}

/**
 * Returns true if the record matches the given filter set.
 *
 * Filter shape:
 *   {
 *     kind?:       "action" | "ack" | string   (exact match)
 *     action?:     string                     (exact match on rec.action)
 *     source?:     "watcher" | "narrator" | "system" | string   (exact match on rec.source)
 *     since?:      ISO timestamp              (rec.ts >= since)
 *     until?:      ISO timestamp              (rec.ts <  until)
 *     minPriority?: number 0..1               (rec.priority >= min)
 *     maxPriority?: number 0..1               (rec.priority <= max)
 *     reasonContains?: string                 (rec.reason substring)
 *   }
 *
 * Undefined filter keys are skipped (always pass).
 */
function recordMatches(rec, f) {
  // Reject null, primitives, and arrays. Only "real" object records pass.
  if (!rec || typeof rec !== "object" || Array.isArray(rec)) return false;

  if (f.kind !== undefined && rec.kind !== f.kind) return false;
  if (f.action !== undefined && rec.action !== f.action) return false;
  if (f.source !== undefined && rec.source !== f.source) return false;

  if (f.since !== undefined) {
    const recTs = parseIsoMs(rec.ts);
    const sinceMs = parseIsoMs(f.since);
    if (recTs !== null && sinceMs !== null && recTs < sinceMs) return false;
  }
  if (f.until !== undefined) {
    const recTs = parseIsoMs(rec.ts);
    const untilMs = parseIsoMs(f.until);
    if (recTs !== null && untilMs !== null && recTs >= untilMs) return false;
  }

  if (f.minPriority !== undefined) {
    if (typeof rec.priority !== "number" || rec.priority < f.minPriority) return false;
  }
  if (f.maxPriority !== undefined) {
    if (typeof rec.priority !== "number" || rec.priority > f.maxPriority) return false;
  }

  if (f.reasonContains !== undefined) {
    if (typeof rec.reason !== "string") return false;
    if (!rec.reason.includes(f.reasonContains)) return false;
  }

  return true;
}

// ---------------------------------------------------------------------------
// A2aQuery
// ---------------------------------------------------------------------------

/**
 * @param {object}  [opts]
 * @param {string}  [opts.dir]            Directory to scan (default ./logs/a2a).
 * @param {string}  [opts.namePrefix]     Filename prefix (default "a2a").
 * @param {boolean} [opts.verbose]        Log skipped/corrupt lines to stderr.
 */
class A2aQuery {
  constructor(opts = {}) {
    this._dir        = path.resolve(opts.dir ?? DEFAULT_DIR);
    this._namePrefix = opts.namePrefix ?? NAME_PREFIX;
    this._verbose    = Boolean(opts.verbose);
  }

  /**
   * Enumerate log filenames in the directory, sorted oldest-first by mtime.
   * We use mtime rather than filename lexicographic order because the
   * A2aLog timestamps include milliseconds and sort correctly by mtime
   * even across rotations within the same second.
   */
  async _listLogFiles() {
    let entries;
    try {
      entries = await fsReaddir(this._dir);
    } catch (err) {
      if (err && err.code === "ENOENT") return [];
      throw err;
    }
    const candidates = entries.filter((n) =>
      n.startsWith(this._namePrefix) && n.endsWith(".jsonl")
    );
    // Annotate with mtime, sort ascending.
    const withMtime = await Promise.all(
      candidates.map(async (name) => {
        const full = path.join(this._dir, name);
        try {
          const st = await fsStat(full);
          return { name, mtimeMs: st.mtimeMs };
        } catch (_) {
          return null; // file vanished or unreadable; skip
        }
      })
    );
    return withMtime
      .filter(Boolean)
      .sort((a, b) => a.mtimeMs - b.mtimeMs)
      .map((x) => x.name);
  }

  /**
   * Stream all records in the directory (oldest-first) and apply a filter.
   * @param {object}  [filters]    See recordMatches() for the shape.
   * @returns {Promise<object[]>}  All matching records.
   */
  async query(filters = {}) {
    const out = [];
    for await (const rec of this._iterate(filters)) {
      out.push(rec);
    }
    return out;
  }

  /**
   * Async generator: yields records one at a time as we read each file.
   * Memory-bounded by line length, not file size.
   *
   * Yields records that pass the filter. Skips malformed JSON lines
   * silently (or warns in verbose mode).
   *
   * @param {object} [filters]
   */
  async *_iterate(filters = {}) {
    const files = await this._listLogFiles();
    for (const fname of files) {
      const full = path.join(this._dir, fname);
      let content;
      try {
        content = await fs.promises.readFile(full, "utf8");
      } catch (err) {
        if (this._verbose) {
          process.stderr.write(`[a2aQuery] skip ${fname}: ${err.message}\n`);
        }
        continue;
      }
      // Split on newlines, skip empties, parse each.
      const lines = content.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        if (!line) continue;
        let rec;
        try {
          rec = JSON.parse(line);
        } catch (_) {
          if (this._verbose) {
            process.stderr.write(`[a2aQuery] skip malformed line in ${fname}:${i + 1}\n`);
          }
          continue;
        }
        if (recordMatches(rec, filters)) yield rec;
      }
    }
  }

  /**
   * Count records by a chosen field. Returns a Map<key, count>.
   *
   * @param {object}  opts
   * @param {string}  opts.field       Record field to group by (e.g. "action", "kind").
   * @param {object}  [opts.filters]   Standard filter set.
   * @returns {Promise<Map<string, number>>}
   */
  async countBy({ field, filters = {} }) {
    if (typeof field !== "string" || !field) {
      throw new Error("A2aQuery.countBy: opts.field (non-empty string) is required");
    }
    const counts = new Map();
    for await (const rec of this._iterate(filters)) {
      const key = rec[field];
      // Skip records that don't have the field at all.
      if (key === undefined || key === null) continue;
      // Coerce non-string keys to string for Map consistency.
      const k = typeof key === "string" ? key : String(key);
      counts.set(k, (counts.get(k) || 0) + 1);
    }
    return counts;
  }

  /**
   * Top-N most-frequent values of a field. Convenience wrapper over countBy.
   *
   * @param {object}  opts
   * @param {string}  opts.field       Field to group by.
   * @param {number}  [opts.limit=10]  Max items to return.
   * @param {object}  [opts.filters]   Standard filter set.
   * @returns {Promise<Array<{key: string, count: number}>>}  Sorted desc by count.
   */
  async topBy({ field, limit = 10, filters = {} } = {}) {
    if (typeof field !== "string" || !field) {
      throw new Error("A2aQuery.topBy: opts.field (non-empty string) is required");
    }
    const counts = await this.countBy({ field, filters });
    const sorted = [...counts.entries()]
      .map(([key, count]) => ({ key, count }))
      .sort((a, b) => b.count - a.count);
    return sorted.slice(0, Math.max(0, limit));
  }

  /**
   * Query records filtered by source provenance.
   *
   * Convenience for `query({ source, ...filters })`. The `source` field is
   * stamped on every A2A record at emission:
   *   - "watcher"   - fired by WatcherRegistry (deterministic predicates)
   *   - "narrator"  - proposed by the LLM narrator
   *   - "system"    - default; replayed or synthesised records
   *
   * @param {string}  source    Source string to match (exact, case-sensitive).
   * @param {object}  [opts]
   * @param {object}  [opts.filters]   Additional standard filter set
   *                                    (e.g. { since, until, action }).
   * @param {number}  [opts.limit]     Cap on returned records (0 = no cap).
   * @returns {Promise<object[]>}
   */
  async bySource(source, { filters = {}, limit = 0 } = {}) {
    if (typeof source !== "string" || !source) {
      throw new Error("A2aQuery.bySource: source (non-empty string) is required");
    }
    if (filters && typeof filters !== "object" && !Array.isArray(filters)) {
      throw new Error("A2aQuery.bySource: filters must be a plain object");
    }
    const results = await this.query({ ...filters, source });
    return limit > 0 ? results.slice(0, limit) : results;
  }

  /**
   * Distribution of records by `source` field.
   *
   * Returns a map { source -> count } across all matching records. Useful
   * for "what fraction of today's alerts were watcher-fired vs LLM?".
   *
   * @param {object}  [filters]   Standard filter set (no source filter applied).
   * @returns {Promise<Map<string, number>>}
   */
  async sourceBreakdown(filters = {}) {
    return this.countBy({ field: "source", filters });
  }

  /**
   * Find the time span of records matching a filter set.
   *
   * Useful for dashboards ("when did the last incident start?", "how
   * long has the vessel been in anomaly mode?"). Iterates the log
   * once, yielding records one at a time, so memory stays bounded
   * regardless of corpus size.
   *
   * Records whose `ts` cannot be parsed as ISO are silently skipped.
   *
   * @param {object}  [filters]   Standard filter set.
   * @returns {Promise<{
   *   earliest: string|null,
   *   latest:   string|null,
   *   spanMs:   number|null,
   *   matched:  number,
   * }>}
   *   `spanMs` is `latest - earliest` in ms, or null if fewer than 2
   *   records had a parseable timestamp. `matched` is the count of
   *   records that passed the filter (NOT the count of records that
   *   had a valid ts; records with invalid ts still increment
   *   `matched` but are excluded from earliest/latest).
   */
  async timeRange(filters = {}) {
    let earliestMs = Infinity;
    let latestMs   = -Infinity;
    let matched    = 0;

    for await (const rec of this._iterate(filters)) {
      matched++;
      const recMs = parseIsoMs(rec.ts);
      if (recMs === null) continue;
      if (recMs < earliestMs) earliestMs = recMs;
      if (recMs > latestMs)   latestMs   = recMs;
    }

    const earliest = earliestMs === Infinity ? null : new Date(earliestMs).toISOString();
    const latest   = latestMs   === -Infinity ? null : new Date(latestMs).toISOString();
    let spanMs = null;
    if (earliestMs !== Infinity && latestMs !== -Infinity && latestMs >= earliestMs) {
      spanMs = latestMs - earliestMs;
    }
    return { earliest, latest, spanMs, matched };
  }

  /**
   * Extract a single numeric field from every record matching the filter set,
   * returning a sorted ascending array of finite numbers.
   *
   * This is the foundation for any statistical analysis (percentiles, mean,
   * std-dev, anomaly detection on a metric). Records without the field, or
   * with a non-finite value, are silently skipped. Use `filter` to invert
   * that behavior (treat missing as a sentinel or return null).
   *
   * @param {string}  field        JSON path (e.g. "priority", "elapsedMs").
   *                               Dot notation supported one level (e.g.
   *                               "payload.depth" -> rec.payload.depth).
   * @param {object}  [filters]    Standard filter set.
   * @param {object}  [opts]
   * @param {boolean} [opts.desc]  Sort descending instead of ascending. Default false.
   * @returns {Promise<number[]>}
   */
  async values(field, filters = {}, opts = {}) {
    if (typeof field !== "string" || field.length === 0) {
      throw new Error("A2aQuery.values: field (non-empty string) is required");
    }
    const desc = opts.desc === true;
    const get = makeFieldGetter(field);
    const out = [];
    for await (const rec of this._iterate(filters)) {
      const v = get(rec);
      if (typeof v !== "number" || !Number.isFinite(v)) continue;
      out.push(v);
    }
    out.sort((a, b) => desc ? b - a : a - b);
    return out;
  }

  /**
   * Single-value percentile of a numeric field, in the range [0, 100].
   *
   * Linear interpolation between sample points (the "C=1" percentile method,
   * which is what numpy's default uses). Returns null if fewer than 1 sample
   * is available.
   *
   * @param {string}  field       JSON path.
   * @param {number}  p           Percentile in [0, 100]. 50 = median, 95 = p95.
   * @param {object}  [filters]   Standard filter set.
   * @returns {Promise<number|null>}
   */
  async percentile(field, p, filters = {}) {
    if (typeof field !== "string" || field.length === 0) {
      throw new Error("A2aQuery.percentile: field (non-empty string) is required");
    }
    if (typeof p !== "number" || !Number.isFinite(p) || p < 0 || p > 100) {
      throw new Error("A2aQuery.percentile: p (number in [0, 100]) is required");
    }
    const samples = await this.values(field, filters);
    if (samples.length === 0) return null;
    if (samples.length === 1) return samples[0];
    const rank = (p / 100) * (samples.length - 1);
    const lo = Math.floor(rank);
    const hi = Math.ceil(rank);
    if (lo === hi) return samples[lo];
    const frac = rank - lo;
    return samples[lo] * (1 - frac) + samples[hi] * frac;
  }

  /**
   * Convenience summary for a numeric field: count, min, max, mean, p50, p95, p99.
   *
   * Designed for dashboards that need to show "what's the latency / priority /
   * depth distribution over the last hour?" without writing the same loop
   * five times. Returns null if no records pass the filter.
   *
   * @param {string}  field
   * @param {object}  [filters]
   * @returns {Promise<{
   *   count: number, min: number|null, max: number|null,
   *   mean: number|null, p50: number|null, p95: number|null, p99: number|null
   * }>}
   */
  async valuesSummary(field, filters = {}) {
    const samples = await this.values(field, filters);
    if (samples.length === 0) {
      return { count: 0, min: null, max: null, mean: null, p50: null, p95: null, p99: null };
    }
    const min = samples[0];
    const max = samples[samples.length - 1];
    let sum = 0;
    for (const v of samples) sum += v;
    const mean = sum / samples.length;
    const pct = (p) => {
      if (samples.length === 1) return samples[0];
      const rank = (p / 100) * (samples.length - 1);
      const lo = Math.floor(rank);
      const hi = Math.ceil(rank);
      if (lo === hi) return samples[lo];
      const frac = rank - lo;
      return samples[lo] * (1 - frac) + samples[hi] * frac;
    };
    return {
      count: samples.length,
      min,
      max,
      mean,
      p50: pct(50),
      p95: pct(95),
      p99: pct(99),
    };
  }

  /**
   * Bucket records by time interval. Each bucket holds the count of records
   * whose `rec.ts` falls within [bucketStart, bucketStart+intervalMs).
   *
   * Useful for "how many A2A actions fired per minute/hour/day?".
   *
   * @param {object}  opts
   * @param {number}  opts.intervalMs   Bucket size in milliseconds.
   * @param {object}  [opts.filters]    Standard filter set.
   * @returns {Promise<Array<{ts: string, count: number}>>}
   *          Sorted by ts ascending. ts is the bucket start as ISO.
   */
  async bucketBy({ intervalMs, filters = {} } = {}) {
    if (!Number.isInteger(intervalMs) || intervalMs <= 0) {
      throw new Error("A2aQuery.bucketBy: opts.intervalMs (positive integer) is required");
    }
    const buckets = new Map(); // bucketStartMs -> count
    for await (const rec of this._iterate(filters)) {
      const recMs = parseIsoMs(rec.ts);
      if (recMs === null) continue;
      const bucketStart = Math.floor(recMs / intervalMs) * intervalMs;
      buckets.set(bucketStart, (buckets.get(bucketStart) || 0) + 1);
    }
    return [...buckets.entries()]
      .sort((a, b) => a[0] - b[0])
      .map(([bucketStartMs, count]) => ({
        ts: new Date(bucketStartMs).toISOString(),
        count,
      }));
  }

  /**
   * Roll-up summary of the log under a filter set.
   *
   * @param {object}  [filters]
   * @returns {Promise<{
   *   totalRecords: number,
   *   byKind:       Map<string, number>,
   *   byAction:     Map<string, number>,
   *   timeRange:    { earliest: string|null, latest: string|null },
   * }>}
   */
  async summary(filters = {}) {
    let total = 0;
    const byKind   = new Map();
    const byAction = new Map();
    let earliestMs = Infinity;
    let latestMs   = -Infinity;

    for await (const rec of this._iterate(filters)) {
      total++;
      if (typeof rec.kind === "string") {
        byKind.set(rec.kind, (byKind.get(rec.kind) || 0) + 1);
      }
      if (typeof rec.action === "string") {
        byAction.set(rec.action, (byAction.get(rec.action) || 0) + 1);
      }
      const recMs = parseIsoMs(rec.ts);
      if (recMs !== null) {
        if (recMs < earliestMs) earliestMs = recMs;
        if (recMs > latestMs)   latestMs   = recMs;
      }
    }

    return {
      totalRecords: total,
      byKind,
      byAction,
      timeRange: {
        earliest: earliestMs === Infinity ? null : new Date(earliestMs).toISOString(),
        latest:   latestMs   === -Infinity ? null : new Date(latestMs).toISOString(),
      },
    };
  }
}

module.exports = {
  A2aQuery,
  // Exported for tests / external reuse.
  recordMatches,
  isA2aLogFilename,
};