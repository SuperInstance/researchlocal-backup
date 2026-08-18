/**
 * tests/a2aQuery.values.test.js
 * ----------------------------------------------------------------------------
 * Tests for the new statistical methods on A2aQuery:
 *   - values(field, filters, opts)       — sorted numeric array
 *   - percentile(field, p, filters)      — single percentile
 *   - valuesSummary(field, filters)      — count, min, max, mean, p50, p95, p99
 * ----------------------------------------------------------------------------
 */
"use strict";

const fs = require("fs");
const os = require("os");
const path = require("path");

const { A2aQuery } = require("../backend/a2aQuery");
const { test, assertEq, assert, assertNear, run } = require("./_harness");

const _harness = require("./_harness");

/**
 * Build a temp dir for fixture isolation. The harness intentionally creates
 * many of these per suite (one per test).
 */
function tmpDir() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "a2aQuery-values-"));
}

/**
 * Write a single-file JSONL log. We pass backdated mtime so mtime-based
 * ordering (which A2aQuery uses) is deterministic.
 */
function writeLog(dir, records, opts = {}) {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  const baseMs = opts.mtimeMs || Date.parse("2026-07-25T12:00:00Z");
  const fname = `a2a-${new Date(baseMs).toISOString()}.jsonl`;
  const fpath = path.join(dir, fname);
  const lines = records.map((r) => JSON.stringify(r)).join("\n") + "\n";
  fs.writeFileSync(fpath, lines);
  const ts = new Date(baseMs);
  fs.utimesSync(fpath, ts, ts);
  return fpath;
}

async function main() {
  // -----------------------------------------------------------------------
  // values(field, filters, opts)
  // -----------------------------------------------------------------------
  section("values(field, filters, opts)");

  await test("values: rejects empty field", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      let thrown = null;
      try { await q.values(""); } catch (e) { thrown = e; }
      assert(thrown, "expected throw on empty field");
      assert(thrown.message.includes("field"));
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: rejects non-string field", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      let thrown = null;
      try { await q.values(42); } catch (e) { thrown = e; }
      assert(thrown, "expected throw on non-string field");
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: empty log returns empty array", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      const out = await q.values("priority");
      assertEq(out.length, 0);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: extracts priority, sorted ascending by default", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.7, ts: new Date(baseMs).toISOString() },
        { kind: "action", action: "announce", priority: 0.2, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.9, ts: new Date(baseMs + 2).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.5, ts: new Date(baseMs + 3).toISOString() },
        { kind: "action", action: "announce", priority: 0.1, ts: new Date(baseMs + 4).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const out = await q.values("priority");
      assertEq(out.length, 5);
      assertEq(out[0], 0.1);
      assertEq(out[1], 0.2);
      assertEq(out[2], 0.5);
      assertEq(out[3], 0.7);
      assertEq(out[4], 0.9);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: opts.desc sorts descending", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.5, ts: new Date(baseMs).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.9, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.1, ts: new Date(baseMs + 2).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const out = await q.values("priority", {}, { desc: true });
      assertEq(out[0], 0.9);
      assertEq(out[2], 0.1);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: skips records missing the field", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.5, ts: new Date(baseMs).toISOString() },
        { kind: "ack", action_id: 1, ts: new Date(baseMs + 1).toISOString() }, // no priority
        { kind: "action", action: "announce", priority: 0.3, ts: new Date(baseMs + 2).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const out = await q.values("priority");
      assertEq(out.length, 2);
      assertEq(out[0], 0.3);
      assertEq(out[1], 0.5);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: skips non-finite priority (NaN, Infinity, strings)", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.5, ts: new Date(baseMs).toISOString() },
        { kind: "action", action: "raise_alert", priority: Number.NaN, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "raise_alert", priority: Number.POSITIVE_INFINITY, ts: new Date(baseMs + 2).toISOString() },
        { kind: "action", action: "raise_alert", priority: "high", ts: new Date(baseMs + 3).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.7, ts: new Date(baseMs + 4).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const out = await q.values("priority");
      assertEq(out.length, 2);
      assertEq(out[0], 0.5);
      assertEq(out[1], 0.7);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: filters compose with field extraction", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.9, ts: new Date(baseMs).toISOString() },
        { kind: "action", action: "announce", priority: 0.2, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.7, ts: new Date(baseMs + 2).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const out = await q.values("priority", { action: "raise_alert" });
      assertEq(out.length, 2);
      assertEq(out[0], 0.7);
      assertEq(out[1], 0.9);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("values: dot-notation field (payload.depth)", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.9,
          payload: { depth: 3.2 }, ts: new Date(baseMs).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.7,
          payload: { depth: 1.5 }, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.5,
          payload: {}, ts: new Date(baseMs + 2).toISOString() }, // no depth
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const out = await q.values("payload.depth");
      assertEq(out.length, 2);
      assertEq(out[0], 1.5);
      assertEq(out[1], 3.2);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  // -----------------------------------------------------------------------
  // percentile(field, p, filters)
  // -----------------------------------------------------------------------
  section("percentile(field, p, filters)");

  await test("percentile: rejects non-string field", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      let thrown = null;
      try { await q.percentile(42, 50); } catch (e) { thrown = e; }
      assert(thrown, "expected throw on non-string field");
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("percentile: rejects p out of [0, 100]", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      let thrown = null;
      try { await q.percentile("priority", 150); } catch (e) { thrown = e; }
      assert(thrown, "expected throw on p > 100");
      thrown = null;
      try { await q.percentile("priority", -1); } catch (e) { thrown = e; }
      assert(thrown, "expected throw on p < 0");
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("percentile: empty log returns null", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      const out = await q.percentile("priority", 50);
      assertEq(out, null);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("percentile: single sample returns that value", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.42, ts: new Date(baseMs).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      assertEq(await q.percentile("priority", 50), 0.42);
      assertEq(await q.percentile("priority", 99), 0.42);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("percentile: median of [1..5] is 3", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [1, 2, 3, 4, 5].map((v, i) => ({
        kind: "action",
        action: "raise_alert",
        priority: v,
        ts: new Date(baseMs + i).toISOString(),
      }));
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      assertEq(await q.percentile("priority", 50), 3);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("percentile: linear interpolation between samples", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      // 0,10,20,30,40,50,60,70,80,90
      const records = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90].map((v, i) => ({
        kind: "action", action: "raise_alert", priority: v,
        ts: new Date(baseMs + i).toISOString(),
      }));
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      // 9 samples means rank = p/100 * 9
      // p=0 -> 0
      assertEq(await q.percentile("priority", 0), 0);
      // p=100 -> 90
      assertEq(await q.percentile("priority", 100), 90);
      // p=50 -> rank 4.5 -> between [40] and [50] = 45
      assertNear(await q.percentile("priority", 50), 45, 1e-9);
      // p=95 -> rank 8.55 -> between [80] and [90] = 85.5
      assertNear(await q.percentile("priority", 95), 85.5, 1e-9);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  // -----------------------------------------------------------------------
  // valuesSummary(field, filters)
  // -----------------------------------------------------------------------
  section("valuesSummary(field, filters)");

  await test("valuesSummary: empty log returns nulls", async () => {
    const dir = tmpDir();
    try {
      const q = new A2aQuery({ dir });
      const s = await q.valuesSummary("priority");
      assertEq(s.count, 0);
      assertEq(s.min, null);
      assertEq(s.max, null);
      assertEq(s.mean, null);
      assertEq(s.p50, null);
      assertEq(s.p95, null);
      assertEq(s.p99, null);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("valuesSummary: count, min, max, mean for a known set", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [1, 2, 3, 4, 5].map((v, i) => ({
        kind: "action", action: "raise_alert", priority: v,
        ts: new Date(baseMs + i).toISOString(),
      }));
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const s = await q.valuesSummary("priority");
      assertEq(s.count, 5);
      assertEq(s.min, 1);
      assertEq(s.max, 5);
      assertEq(s.mean, 3);
      assertEq(s.p50, 3);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("valuesSummary: respects filters", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.9, ts: new Date(baseMs).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.7, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "announce", priority: 0.1, ts: new Date(baseMs + 2).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const s = await q.valuesSummary("priority", { action: "raise_alert" });
      assertEq(s.count, 2);
      assertEq(s.min, 0.7);
      assertEq(s.max, 0.9);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });

  await test("valuesSummary: skip records missing the field", async () => {
    const dir = tmpDir();
    try {
      const baseMs = Date.parse("2026-07-25T12:00:00Z");
      const records = [
        { kind: "action", action: "raise_alert", priority: 0.5, ts: new Date(baseMs).toISOString() },
        { kind: "ack", action_id: 1, ts: new Date(baseMs + 1).toISOString() },
        { kind: "action", action: "raise_alert", priority: 0.7, ts: new Date(baseMs + 2).toISOString() },
      ];
      writeLog(dir, records, { mtimeMs: baseMs });
      const q = new A2aQuery({ dir });
      const s = await q.valuesSummary("priority");
      assertEq(s.count, 2);
    } finally {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch (_) {}
    }
  });
}

function section(name) { console.log("\n--- " + name + " ---"); }

run("a2aQuery.values", async () => { await main(); });
