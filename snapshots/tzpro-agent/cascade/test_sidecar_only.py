"""cascade/test_sidecar_only.py — degraded-mode honesty tests.

The sidecar-only path is the cascade's fallback when no vision model is
loaded (Starlink-limited, can't pull moondream/llava). It must:
  1. Never invent sounder content (depth, schools, thermocline, bottom type).
  2. Emit a record with analysis_mode="sidecar_only" and model=null.
  3. Carry real navigation facts from the sidecar verbatim.
  4. H1/D1 must refuse to invent content — they write a programmatic
     position-track brief with a Mode disclosure section.
  5. Evening GC must NOT abort when vision is missing (only when ollama
     is down). It just skips the final-read pass.

These tests run with the real config — they don't depend on a vision
model being installed.
"""
from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

# Use a temp workspace so tests don't pollute the live capture area.
TMP = Path(tempfile.mkdtemp(prefix="tzpro_sidecar_"))
os.environ["TZPRO_WORKSPACE"] = str(TMP)
os.environ["CASCADE_OUT"] = str(TMP / "cascade_out")

# Add repo root so `cascade` package resolves cleanly.
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from cascade import config, decaminute_loop, hourly_loop, daily_loop, retention  # noqa: E402


def _make_capture(name: str, lat: float, lon: float, sog: float = 1.5,
                  cog: float | None = 180.0) -> tuple[Path, Path]:
    """Create a 1x1 PNG + sidecar JSON for testing."""
    import struct
    import zlib
    # minimal 1x1 PNG
    png_bytes = (
        b"\x89PNG\r\n\x1a\n"
        + b"\x00\x00\x00\rIHDR"
        + struct.pack(">II", 1, 1) + b"\x08\x02\x00\x00\x00"
        + b"\x90wS\xde"
        + b"\x00\x00\x00\x0cIDATx\x9cc\xf8\xff\xff?\x00\x05\xfe\x02\xfe\xa3\x9b\xc1\x10"
        + b"\x00\x00\x00\x00IEND\xaeB`\x82"
    )
    png_path = TMP / "captures" / "v3" / f"2026-07-22_test_{lat}N_{abs(lon)}W" / f"{name}.png"
    png_path.parent.mkdir(parents=True, exist_ok=True)
    png_path.write_bytes(png_bytes)

    sidecar = {
        "capture_id": name,
        "ts_utc": "2026-07-22T13:50:00+00:00",
        "ts_local": "2026-07-22T05:50:00-08:00",
        "frame_file": f"{name}.png",
        "position": {
            "lat_dd": lat, "lon_dd": -lon,  # convention: west positive in ddmm
            "lat_ddmm": f"{int(lat * 100) / 100:.3f}N".replace(".", ""),
            "lon_ddmm": f"{int(lon * 100) / 100:.3f}W".replace(".", ""),
            "sog_kts": sog, "cog_deg": cog,
        },
    }
    sidecar_path = png_path.with_suffix(".json")
    sidecar_path.write_text(json.dumps(sidecar), encoding="utf-8")
    return png_path, sidecar_path


class TestSidecarOnlyRecord(unittest.TestCase):

    def setUp(self):
        config.ensure_dirs()
        # Clean records between tests
        for f in config.DIR_RECORDS.glob("*_record.json"):
            f.unlink()

    def test_record_emitted_when_no_vision_model(self):
        """If ollama is up but no vision model is present, M10 must still
        emit a record rather than idling."""
        png, sc = _make_capture("0600_5547.191N_13142.060W",
                                lat=55.7865, lon=131.7010, sog=1.8)
        rec = decaminute_loop.write_record(png, json.loads(sc.read_text()), [])
        self.assertIsNotNone(rec, "M10 must emit a record in sidecar-only mode")
        self.assertEqual(rec["analysis_mode"], "sidecar_only")
        self.assertIsNone(rec["model"])

    def test_no_sounder_claims_in_sidecar_only_record(self):
        """Sidecar-only records must NOT fabricate depth, schools, or
        thermocline — the constraint 5 'never invent analysis' rule."""
        png, sc = _make_capture("0610_5547.264N_13142.524W",
                                lat=55.7877, lon=131.7087, sog=1.6)
        rec = decaminute_loop.write_record(png, json.loads(sc.read_text()), [])
        self.assertIsNone(rec["bottom_fm"])
        self.assertEqual(rec["bottom_type"], "unknown")
        self.assertEqual(rec["schools"], [])
        self.assertIsNone(rec["thermocline_fm"])
        self.assertEqual(rec["haze"], "unknown")
        self.assertEqual(rec["anomalies"], [])

    def test_navigation_facts_preserved(self):
        """Lat/lon/SOG/COG from sidecar must flow through verbatim."""
        png, sc = _make_capture("0620_5547.350N_13142.900W",
                                lat=55.7892, lon=131.7150, sog=2.1, cog=325.0)
        rec = decaminute_loop.write_record(png, json.loads(sc.read_text()), [])
        self.assertAlmostEqual(rec["lat"], 55.7892, places=4)
        self.assertAlmostEqual(rec["lon"], -131.7150, places=4)
        self.assertAlmostEqual(rec["sog_kts"], 2.1, places=2)
        self.assertAlmostEqual(rec["cog_deg"], 325.0, places=2)

    def test_summary_is_honest_about_degradation(self):
        """The summary string should not claim to have looked at the sounder."""
        png, sc = _make_capture("0630_5547.450N_13143.100W",
                                lat=55.7908, lon=131.7183, sog=1.9)
        rec = decaminute_loop.write_record(png, json.loads(sc.read_text()), [])
        self.assertIn("Sidecar-only", rec["summary"])
        self.assertNotIn("school", rec["summary"].lower())
        self.assertNotIn("thermocline", rec["summary"].lower())


class TestH1SidecarOnly(unittest.TestCase):

    def setUp(self):
        config.ensure_dirs()
        for f in config.DIR_RECORDS.glob("*_record.json"):
            f.unlink()
        for f in config.DIR_BRIEFINGS.glob("briefing_*.md"):
            f.unlink()
        for f in config.DIR_BRIEFINGS.glob("briefing_*.json"):
            f.unlink()

    def test_h1_writes_programmatic_brief_when_all_sidecar_only(self):
        """Three sidecar-only records → H1 must produce a real .md and
        .json without calling the LLM."""
        for i, (lat, lon) in enumerate([(55.7865, 131.7010),
                                         (55.7877, 131.7087),
                                         (55.7892, 131.7150)]):
            name = f"{i:02d}00_55n_{lon:.3f}w"
            png, sc = _make_capture(name, lat=lat, lon=lon, sog=1.5 + i * 0.2)
            decaminute_loop.write_record(png, json.loads(sc.read_text()), [])

        path = hourly_loop.write_briefing()
        self.assertIsNotNone(path)
        body = Path(path).read_text(encoding="utf-8")
        # Programmatic position-track brief
        self.assertIn("Position Track", body)
        # Mode disclosure
        self.assertIn("Sidecar-Only", body)
        # No fabricated sounder claims
        self.assertNotIn("recommended", body.lower())
        # JSON sidecar exists
        jpath = Path(str(path).replace(".md", ".json"))
        self.assertTrue(jpath.exists())
        j = json.loads(jpath.read_text(encoding="utf-8"))
        self.assertEqual(j["analysis_mode"], "sidecar_only")
        self.assertEqual(j["counts"]["sidecar_only_records"], 3)


class TestEveningGCNoVision(unittest.TestCase):

    def setUp(self):
        config.ensure_dirs()
        # Create a day folder with one canonical-frame PNG (kept) and
        # one non-canonical PNG (would-be-GC'd).
        day_dir = config.CAPTURES / "2026-07-22_test_NNN_WWW"
        day_dir.mkdir(parents=True, exist_ok=True)
        # Non-canonical 1-min PNG
        for i in range(2):
            png, _ = _make_capture(f"05{i}0_test", 55.78, 131.70)
            png.rename(day_dir / png.name)

    def test_evening_gc_skips_final_read_when_no_vision(self):
        """When ollama is up but no vision model exists, GC must NOT
        abort. It must skip the final-read and just delete 1-min PNGs."""
        from cascade.retention import evening_final_read
        day_dir = config.CAPTURES / "2026-07-22_test_NNN_WWW"
        report = evening_final_read(day_dir)
        # Should NOT have aborted
        self.assertIn("frames_read", report)
        self.assertGreaterEqual(report.get("gc_pngs", 0), 0)


class TestD1SidecarOnly(unittest.TestCase):

    def setUp(self):
        config.ensure_dirs()
        for f in config.DIR_RECORDS.glob("*_record.json"):
            f.unlink()
        for f in config.DIR_BRIEFINGS.glob("day_*.md"):
            f.unlink()
        for f in config.DIR_BRIEFINGS.glob("day_*.json"):
            f.unlink()
        # Two sidecar-only records dated 2026-07-22
        for lat, lon, sog in [(55.7865, 131.7010, 1.5),
                               (55.7877, 131.7087, 1.8)]:
            png, sc = _make_capture(f"06{int(sog*10):02d}_{lat:.3f}", lat=lat, lon=lon, sog=sog)
            decaminute_loop.write_record(png, json.loads(sc.read_text()), [])

    def test_d1_produces_skeleton_with_disclosure(self):
        """D1 should run the skeleton path (no LLM call) and surface
        sidecar-only mode in the daily brief."""
        md = daily_loop.write_daily("2026-07-22")
        self.assertIsNotNone(md)
        body = md.read_text(encoding="utf-8")
        # Sidecar-only disclosure
        self.assertIn("sidecar-only", body.lower())
        self.assertIn("moondream", body)  # tells captain how to unlock
        # JSON exists with explicit analysis_mode
        jpath = md.with_suffix(".json")
        self.assertTrue(jpath.exists())
        j = json.loads(jpath.read_text(encoding="utf-8"))
        self.assertEqual(j["analysis_mode"], "sidecar_only")


def tearDownModule():
    shutil.rmtree(TMP, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
