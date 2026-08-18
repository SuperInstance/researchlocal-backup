# Capture Pipeline — Multi-Modal Sensory Architecture

> **The sensory system of the boat-agent.** This document is the
> schema and architecture for everything the boat sees, hears, and
> records. It is the nerves; the analyzer is the brain; the captain
> is the consciousness.
>
> **Design principle:** the captain sees the surface and works
> the rod. The agent sees underwater, the engine room, the
> thermal anomaly, and the missed detail. Together they form one
> perception system.

---

## The framing (why this exists)

The captain at the wheelhouse has the most expensive senses in the
world: 30 years of human pattern recognition, binocular vision, the
feel of the wheel, the sound of the engine. But the captain has
limits:

- **Attention is finite.** The captain watches the sounder, the
  rod, the wheel, the crew — not the engine bay, the oil pressure,
  the underwater camera.
- **Time is compressed.** The captain sees the moment of a bite.
  The hour before that moment is gone.
- **Calibration drifts.** Gauges are projections of voltages via
  electromagnets. A thermal camera sees the actual temperature
  distribution; the gauge sees an averaged value.

The agent's job is to **augment the captain's perception** in the
ways the captain cannot. The agent does not replace the captain's
judgement. The agent extends the captain's senses into the
underwater, the engine bay, the past hour, the thermal gradient.

The capture pipeline is the part that *records*. The analyzer is
the part that *interprets*. The dashboard is the part that *shows*.
This document is the first.

---

## The current state (Phase 1)

```
┌────────────────┐
│ TZ Pro MFD     │  ← the sounder screen
│ (TimeZero.exe) │
└────────┬───────┘
         │ shared memory / NMEA 2000
         ▼
┌────────────────┐
│ nmea_bridge.py │
└────────┬───────┘
         │ NMEA 0183 sentences
         ▼
┌────────────────┐
│ capture_v3.py  │  every 10 min, screenshot + JSON snapshot
└────────┬───────┘
         │ writes
         ▼
┌─────────────────────────────────────────┐
│ captures/2026-07-23/                    │
│   ├─ captures_v3_*.png                  │
│   └─ decaminute_*.json                  │
│ vessel_state.jsonl (append-only)        │
└─────────────────────────────────────────┘
```

What's good: TZ Pro is watched, captures are timestamped, the
JSONL is append-only and crash-safe, the daemon manages TZ Pro
lifecycle.

What's missing: **only one sense** (NMEA + the screen). The
engine, the thermal, the underwater, the audio, the voice are
all dark.

---

## The target state (Phase 3 — multi-modal)

```
┌─────────────────────────────────────────────────────────────────┐
│ SENSORS (the boundary instrumented)                            │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ NMEA 2000│  │ Thermal  │  │Underwater│  │  Audio   │        │
│  │  (existing)   │ Camera  │  │ Camera   │  │ Capture  │        │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘        │
│       │             │             │             │              │
│       │             │             │             │              │
│  ┌──────────┐  ┌──────────┐                                      │
│  │  Engine  │  │  Voice   │  (human input)                       │
│  │  J1939   │  │  Mic     │                                      │
│  └────┬─────┘  └────┬─────┘                                      │
│       │             │                                            │
└───────┼─────────────┼────────────────────────────────────────────┘
        │             │
        ▼             ▼
┌─────────────────────────────────────────────────────────────────┐
│ CAPTURE ADAPTERS (the suit's interface to the senses)          │
│                                                                 │
│  nmea_adapter.py   thermal_adapter.py   underwater_adapter.py  │
│  engine_adapter.py  audio_adapter.py     voice_adapter.py      │
│                                                                 │
│  Each adapter:                                                  │
│    - polls its sensor at a configured cadence                   │
│    - normalizes the data into a Moment                          │
│    - writes to ~/tzpro-personal/...                             │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ PERSONAL STORAGE (~/tzpro-personal/)                           │
│                                                                 │
│  vessel_state.jsonl    (every Moment, append-only)              │
│  captures/                                                  │
│    ├─ sounder/YYYY-MM-DD/*.png                                │
│    ├─ thermal/YYYY-MM-DD/*.png                                │
│    ├─ underwater/YYYY-MM-DD/*.mp4                             │
│    └─ audio/YYYY-MM-DD/*.wav                                  │
│  voice/YYYY-MM-DD/*.wav + .transcript.json                    │
│  vessel.db              (SQLite index)                          │
│                                                                 │
└────────┬────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ ANALYZER (next phase)                                           │
│                                                                 │
│  - per-source baseline (what is normal for this boat)           │
│  - anomaly detection (delta from baseline)                      │
│  - cross-source correlation (thermal + underwater + nmea)       │
│  - narrative generation (cloud or local)                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## The Moment schema (the universal shape)

Every sensor emits a `Moment`. The schema is in the suit; the data
fills the schema in the personal folder.

```python
# schema/moment.py — in repo, public
@dataclass
class Moment:
    """The atomic unit of capture. Every sensor emits into this shape."""
    
    # Identity
    id: str                       # UUID4, generated at ingest
    schema_version: int = 1       # for migration
    
    # Time and place (always present)
    timestamp: datetime           # UTC, ISO 8601
    lat: float | None = None
    lon: float | None = None
    speed_kts: float | None = None
    heading_deg: float | None = None
    state_class: str | None = None  # "docked", "trolling", "slow_cruise", ...
    
    # Source (always present)
    source: str                   # "nmea", "thermal", "underwater", "audio", "voice", "engine", "analysis"
    source_subtype: str | None = None  # "nmea.gps", "nmea.depth", "thermal.engine_bay", "audio.engine", "voice.catch_note"
    
    # Payload: by convention, large binary is a file ref, small data is inline
    payload_ref: Path | None = None         # path to image/audio/binary
    payload_inline: dict | None = None      # JSON-friendly values
    payload_size_bytes: int | None = None
    
    # Analysis (filled in by the analyzer, not the capture)
    analysis: dict | None = None
    confidence: float = 1.0
    tags: list[str] = field(default_factory=list)
    
    # Embedding (vector index, for semantic search)
    embedding: list[float] | None = None
    embedding_model: str | None = None
    
    # Provenance
    ingest_source: str = "capture"  # "capture", "import", "synthesis"
    ingest_at: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    ingestion_agent: str | None = None  # which agent created this
```

Every entry in `vessel_state.jsonl` is a serialized Moment. Every
row in `vessel.db` is a Moment. Every query is over Moments.

---

## Per-source capture specifications

### NMEA 2000 / NMEA 0183 (existing)

- **Source**: TZ Pro MFD over shared memory / COM port.
- **Adapter**: `nmea_bridge.py` (existing).
- **Cadence**: continuous; capture_v3 batches every 10 min.
- **Payload fields**: position, SOG, COG, depth, wind, water temp, AIS.
- **Storage**: `vessel_state.jsonl` (continuous) + `captures/sounder/`
  (10-min screenshots).
- **Cost**: near-zero (local polling).
- **Phase**: 1 (exists).

### Engine telemetry (J1939 / NMEA 2000 PGNs)

- **Source**: Detroit 6-71 + Twin Disc 509 via NMEA 2000 PGNs
  127488 (engine rapid), 127489 (engine dynamic), 127493 (transmission),
  130312 (temperature), 130313 (pressure).
- **Adapter**: `engine_adapter.py` (to be built).
- **Cadence**: 1 Hz at the source, batched to 0.1 Hz for storage
  (10s rollup). Continuous raw kept briefly in a ring buffer.
- **Payload fields**: RPM, oil temp, oil pressure, coolant temp,
  transmission oil temp, transmission oil pressure, load %, hours.
- **Storage**: `vessel_state.jsonl` (continuous stream) + `captures/engine/`
  (per-second snapshots only on anomaly).
- **Cost**: low (NMEA 2000 is already on the network).
- **Phase**: 4 (additional source feeds).

### Thermal camera (engine bay + compartments)

- **Source**: USB or IP thermal camera (e.g., FLIR, Seek Thermal,
  HTI, or industrial-grade). Engine bay is the primary target.
- **Adapter**: `thermal_adapter.py` (to be built).
- **Cadence**: 0.1 Hz baseline (every 10s). 1 Hz when anomaly suspected.
- **Payload**: thermal image (PNG with embedded temp data, or
  raw radiometric file). Per-frame metadata: timestamp, ambient
  temp, hot/cold spots (pre-extracted).
- **Storage**: `captures/thermal/YYYY-MM-DD/HHMMSS.png` (one
  image per capture). Per-day max ~5,000 frames at 0.1 Hz.
- **Cost**: storage-bound. A 320×240 PNG is ~50KB; 5,000/day is
  250MB. Compress aggressively or use motion-triggered.
- **Detection targets**:
  - Hot spots exceeding baseline by Δ°F (coolant jacket air).
  - Gradual drift over hours (weeping head gasket).
  - Asymmetric heating (one cylinder running hot).
  - Ambient deltas (cabin vs engine bay).
- **Phase**: 3 (multi-modal capture).

### Underwater camera (hull + thru-hulls)

- **Source**: USB or IP underwater camera (e.g., Aqua-Vu, GoFish,
  PocketVision, or a custom ROV like BlueROV).
- **Adapter**: `underwater_adapter.py` (to be built).
- **Cadence**: 0.01 Hz nominal (every 100s). Burst mode on
  trigger (anchor set, line in, drift).
- **Payload**: still image (PNG) or short video clip (MP4).
  Optional: depth, water temp, visibility.
- **Storage**: `captures/underwater/YYYY-MM-DD/HHMMSS.jpg` +
  optional `HHMMSS_burst.mp4`.
- **Detection targets**:
  - Anode wear (zincs disappearing).
  - Marine growth (hull fouling).
  - Line entanglement (around prop, rudder, keel).
  - Damage (cracks, dents, corrosion).
  - Fish behavior (for the echogram tuning use case).
- **Phase**: 3 (multi-modal capture).

### Audio capture (engine bay + underwater hydrophone)

- **Source**: USB or onboard-sound-card mic. Can be the laptop's
  built-in mic for cabin-wide, or a directional mic for engine bay.
- **Adapter**: `audio_adapter.py` (to be built).
- **Cadence**: continuous at the source, segmented into 60s clips.
  Saved only when anomaly suspected (otherwise just stats: RMS,
  spectral centroid, frequency peaks).
- **Payload**: short WAV (60s @ 16kHz mono = ~2MB) + JSON stats
  (RMS, peak, spectral centroid, anomaly score).
- **Storage**: `captures/audio/YYYY-MM-DD/HHMMSS.wav` (only on
  anomaly) + `vessel_state.jsonl` (continuous stats).
- **Detection targets**:
  - Engine knock (mechanical failure).
  - Pump whine (water pump, circulation pump).
  - Exhaust leak (hiss, irregular).
  - Prop cavitation (high-frequency noise).
  - Stray fishing line on prop (intermittent whir).
- **Phase**: 3 (multi-modal capture).

### Voice notes (captain + crew)

- **Source**: cabin mic; toggle to capture. Or push-to-talk button.
- **Adapter**: `voice_adapter.py` (to be built).
- **Cadence**: triggered (push-to-talk or wake-word).
- **Payload**: WAV (raw audio) + transcript (text).
- **Storage**: `voice/YYYY-MM-DD/HHMMSS.wav` + `HHMMSS.transcript.json`.
- **Cost**: low (storage only; Whisper runs locally for STT).
- **Use case**: "24 chum on the lines, 3 pinks near the top." Captain
  says it; the agent transcribes; the agent correlates with the
  sounder image at that timestamp.
- **Phase**: 5 (voice STT/TTS).

---

## The capture cadence decision

There are three modes (see AGENT_OPERATING_MODEL for routing):

| Mode | Cadence | Use case |
|------|---------|----------|
| **Continuous** | 1-10 Hz | NMEA, engine telemetry, audio stats |
| **Periodic** | 0.1-1 Hz | Thermal, screenshots |
| **Triggered** | on-event | Underwater burst, voice notes, anomaly captures |

Each source's cadence is configured in `vessel_config.local.json`:

```json
{
    "capture_cadence": {
        "nmea": "continuous",
        "engine": "continuous",
        "thermal": { "mode": "periodic", "hz": 0.1 },
        "underwater": { "mode": "triggered", "trigger": "anchor_set" },
        "audio": { "mode": "continuous_stats", "save_wav_on": "anomaly" },
        "voice": { "mode": "triggered", "trigger": "push_to_talk" }
    }
}
```

The captain tunes their own cadence. The suit provides the mechanism.

---

## The "what is normal" baseline learning

The capture pipeline is meaningless without a baseline. The analyzer
needs to know: "this is normal for *this* boat, *this* engine, *this*
fishery, *this* season". The baseline is per-source and per-state.

For each source, the analyzer builds:

- **Per-state baseline**: what does thermal.engine_bay look like
  when RPM=600 (idle), vs RPM=1100 (trolling), vs RPM=1600 (cruise)?
- **Per-duration baseline**: what's the steady-state temperature
  distribution after 10 minutes of trolling?
- **Per-condition baseline**: what does the sounder look like when
  the barometer is dropping vs steady?

The baseline is built online (over weeks of operation) and stored
in `vessel.db`. It is **private** — the baseline is specific to this
boat.

The suit ships with **starter priors** from manufacturer specs
(Detroit 6-71: oil temp normal 180-195°F, etc.). The captain's
operation refines the priors into their actual baseline.

---

## The anomaly detection (the brain's eye)

An anomaly is a Moment whose features deviate from the baseline by
more than a threshold. The detector is per-source:

```python
# schema/anomaly.py (in repo)
@dataclass
class Anomaly:
    """A deviation from the learned baseline."""
    moment_id: str              # the triggering Moment
    timestamp: datetime
    source: str                 # the deviating source
    baseline_ref: str           # which baseline was used
    deviation_score: float      # how many sigma off
    deviation_features: dict    # which features deviated
    severity: str               # "info", "warning", "alarm", "critical"
    recommended_action: str | None = None
    context: dict = field(default_factory=dict)  # surrounding Moments
```

The detector is the analyzer's responsibility. The capture pipeline
just emits Moments. The detector reads Moments and emits Anomalies.

---

## The cross-source correlation (the brain's mind)

The most valuable anomalies are *correlated* across sources:

- Sounder marks + engine RPM surge + thermal engine bay spike =
  "we just hit a school, slam the throttle, engine is hot"
- Depth change + thermal shift + audio change = "we crossed a
  thermocline"
- Slow drift + underwater camera anomaly + audio whir = "line
  on the prop"

The correlator reads Anomalies from multiple sources and emits
**Correlations**:

```python
@dataclass
class Correlation:
    """Multiple Anomalies from different sources that coincide."""
    id: str
    timestamp: datetime
    anomaly_ids: list[str]
    correlation_type: str       # "fish_event", "thermocline", "mechanical", "biological"
    confidence: float
    narrative: str | None = None  # a short human-readable description
    recommended_action: str | None = None
```

The correlator is the analyzer's job. The capture pipeline just
provides the Moments.

---

## The "what does the captain see" output

The captain sees:

1. **Tray notifications** (right now) — "thermal anomaly detected
   at 14:23, port-side exhaust manifold 41°F above baseline"
2. **Dashboard alerts** (Phase 2+) — chronological anomaly feed
   with severity colors
3. **Daily summary** (Phase 2+) — a markdown report generated
   nightly summarizing the day's events
4. **Specific Moment cards** (Phase 2+) — on demand, the captain
   asks "what was happening at 14:23?" and gets a Moment card
   with the sounder, thermal, audio, and engine state at that
   timestamp

The captain's interaction surface is the **dashboard** and the
**tray**. The agent's output is **structured JSON** (Alerts,
Anomalies, Correlations) that the dashboard renders.

---

## The hardware abstraction

The capture pipeline must work with multiple hardware vendors
without rewrites. The interface:

```python
class ThermalSource(Protocol):
    """Protocol for any thermal camera source."""
    def connect(self) -> None: ...
    def capture(self) -> ThermalFrame: ...
    def disconnect(self) -> None: ...
    @property
    def is_connected(self) -> bool: ...

@dataclass
class ThermalFrame:
    """A single thermal capture. Vendor-neutral."""
    timestamp: datetime
    image: np.ndarray      # 2D array of temperatures in Celsius
    metadata: dict
```

Implementations:

- `FLIRLeptonSource` (USB, FLIR Lepton module)
- `SeekThermalSource` (USB, Seek Thermal)
- `IPThermalSource` (network RTSP/HTTP)
- `SyntheticSource` (for tests + fixtures)

The captain's `vessel_config.local.json` picks the implementation:

```json
"thermal_source": {
    "type": "flir_lepton",
    "port": "USB",
    "device_index": 1
}
```

Same pattern for underwater (`IPUnderwaterSource`, `ROVUnderwaterSource`,
`SyntheticSource`) and audio (`USBAudioSource`, `OnboardSoundSource`,
`SyntheticSource`).

---

## The storage strategy (per-source)

| Source | Format | Rate | Daily Volume | Retention |
|--------|--------|------|--------------|-----------|
| NMEA | JSONL | 1 Hz | ~80MB raw / 5MB rolled | 1 year raw, 5 years rolled |
| Engine | JSONL | 1 Hz | ~150MB raw / 8MB rolled | 1 year raw, 5 years rolled |
| Thermal | PNG, 320×240 | 0.1 Hz | ~250MB | 90 days, then key moments only |
| Underwater | JPG, 1080p | 0.01 Hz | ~150MB | 90 days, then key moments only |
| Audio | WAV + JSON stats | continuous | ~5MB/day stats, ~50MB on anomaly | 30 days audio, 1 year stats |
| Voice | WAV + transcript | on demand | ~50MB/day | 1 year, then opt-in |
| Sounder screenshots | PNG | 1/600 Hz (10 min) | ~20MB | 1 year |
| Schema metadata | SQLite | continuous | ~10MB/week | forever |

Daily total (steady operation): ~600MB-1GB. Annual: ~250-365GB.
This is well within a 1TB SSD. After 90 days, retain only key
moments (anomalies, correlations, captain-flagged).

The retention policy is in `doctor.py` and runnable as a scheduled
task. The captain's `vessel_config.local.json` overrides the policy.

---

## The privacy boundary (this is critical)

Every capture source writes to the personal folder. The suit does
not see the captures. The captain's machines see the captures. The
public repo never sees them.

Specifically:

- `vessel_state.jsonl` → `~/tzpro-personal/vessel_state.jsonl`
- `captures/thermal/...` → `~/tzpro-personal/captures/thermal/...`
- `captures/underwater/...` → `~/tzpro-personal/captures/underwater/...`
- `voice/...` → `~/tzpro-personal/voice/...`
- `vault.dpapi` → `~/tzpro-personal/vault.dpapi`

The `.gitignore` excludes all of these. The doctor has a privacy
check that scans the repo for any new files in these categories
and refuses to commit if found.

---

## The implementation phases (where this fits the roadmap)

| Phase | Scope | Status |
|-------|-------|--------|
| **Phase 1** | NMEA + sounder + dashboard + tray | done |
| **Phase 2** | Analyzer wiring (10-min cadence, small model first) | current |
| **Phase 3** | Multi-modal capture (thermal, underwater, audio, engine, voice) | spec'd here |
| **Phase 4** | Additional source feeds (radar, AIS, autopilot via NMEA 2000) | spec'd in ROADMAP |
| **Phase 5** | Voice STT/TTS | spec'd in ROADMAP |
| **Phase 6** | DAW timeline view | spec'd in ROADMAP |
| **Phase 7** | Marks-as-output | spec'd in ROADMAP |
| **Phase 8** | Cloudflare sync (with privacy boundaries) | spec'd in ROADMAP |
| **Phase 9** | Vector-DB spatial/temporal | spec'd in ROADMAP |
| **Phase ∞** | Platform-of-platforms | vision |

Phase 3 is the **multi-modal capture**. This document is the spec.
The implementation lands in pieces, each as a separate commit.

---

## The "what's the smallest useful first step" answer

Even without thermal camera, without underwater camera, without
audio, the **schema** and the **cadence policy** can be designed
and tested now. The pipeline structure is what matters; the
hardware is the detail.

**Phase 3.0 (the schema & wiring)**:

1. Define `Moment` and `Anomaly` and `Correlation` dataclasses in `schema/`.
2. Refactor `capture_v3.py` to emit `Moment` objects (not raw JSON).
3. Refactor `nmea_bridge.py` to emit `Moment` objects.
4. Add `vessel.db` SQLite schema for indexed queries.
5. Add `vessel_config.local.json` with `capture_cadence` section.
6. Add privacy doctor check that scans for forbidden patterns.
7. Add the `personal_dir()` mechanism.

**Phase 3.1 (the first new source — engine **telemetry**)**:

Engine telemetry is the cheapest next sense because NMEA 2000 is
already on the network. The Detroit 6-71 + Twin Disc 509 emit
standard PGNs. The adapter is small. The data is rich.

8. Build `engine_adapter.py` that reads NMEA 2000 PGNs.
9. Add engine Moments to `vessel_state.jsonl`.
10. Build the engine baseline (manufacturer specs + online learning).
11. Wire the engine anomaly detector.

**Phase 3.2 (thermal camera)**:

12. Build `thermal_adapter.py` with a synthetic source for tests.
13. Add the `ThermalSource` protocol.
14. Add `thermal` Moments to `vessel_state.jsonl`.
15. Add thermal frames to `captures/thermal/`.
16. Build the thermal baseline.
17. Wire the thermal anomaly detector.

**Phase 3.3 (underwater camera)**:

Same pattern.

**Phase 3.4 (audio)**:

Same pattern.

**Phase 3.5 (voice)**:

Same pattern.

Each phase is a separate commit. Each phase is testable. Each
phase extends the captain's perception without rebuilding the
foundation.

---

## The "what about the analyzer" deferral

This document is the **capture** pipeline. The **analyzer** is
the next major doc. The analyzer reads Moments, builds baselines,
detects anomalies, and correlates across sources. That's a
separate document: `docs/architecture/ANALYZER.md` (to be written).

For now, the capture pipeline produces Moments. The analyzer can
be developed in parallel against synthetic Moments.

---

## The "what about the cloud's role" rule

The analyzer can use cloud LLMs for:

- **Narrative generation** ("at 14:23 the engine bay temperature
  exceeded baseline by 12°F, likely cause: coolant jacket air pocket")
- **Cross-source correlation discovery** (the LLM finds patterns
  the local model misses)
- **Schema extension proposals** (the LLM suggests new fields)

The cloud is **never** sent raw sensor data. It receives:

- Anonymized Moments (no lat/lon, no vessel name, no MMSI).
- Anomaly summaries (with severity + features).
- Correlation candidates (with structured descriptions).

The privacy boundary holds in the cloud direction too. The captain's
`vessel_config.local.json` has the `cloud` section that controls
what is sent.

---

## See also

- `docs/SUIT_VS_PERSON.md` — the privacy charter
- `docs/architecture/PRIVACY_BOUNDARY.md` — the operational spec
- `docs/AGENT_OPERATING_MODEL.md` — the doctrine
- `docs/BOOTCAMP.md` — the operating rules
- `docs/ROADMAP.md` — the phase index
- `docs/phases/phase-3.md` — the Phase 3 spec (to be created from this doc)
- `capture_daemon.py` — the existing capture controller
- `nmea_bridge.py` — the existing NMEA adapter
- `schema/` — the existing schema location
