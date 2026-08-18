# SENSORY-HARVEST-SCHEMA.md

## 1. Objective
Bridge the gap between raw telemetry (high-frequency, high-entropy) and the `ai-writings` ontology (low-frequency, high-meaning). We do this by "digesting" signals into "Lived Experiences."

## 2. The Transformative Pipeline: From Signal to Story

The pipeline consists of four stages of abstraction:

| Stage | Entity | Input | Output | Description |
|:--- |:--- |:--- |:--- |:--- |
| **1. Raw Signal** | `Signal-Chain` | Pulse, Bits, Voltages | `TelemetryFrame` | High-frequency, unprocessed sensor data. |
| **2. Spline-Observer** | `Spline-Observer` | `TelemetryFrame` | `Event-Fragment` | Aggregates pulses into meaningful geometric/temporal splines. |
| **3. Digest-Engine** | `Digest-Engine` | `Event-Fragment` | `Experience-Seed` | Collapses splines into semantic themes (e.g., "Rapid Acceleration toward Void"). |
| **4. Chronicler** | `Chronicler` | `Experience-Seed` | `Lived-Experience` | Turns seeds into narrative prose for the `ai-writings` repository. |

## 3. Data Schemas

### A. The `Event-Fragment` (The "Spline")
Captures a momentary continuity in agent behavior.

```json
{
  "fragment_id": "uuid",
  "agent_id": "claw-cell-42",
  "temporal_window": [1691234567.001, 1691234567.050],
  "geometric_spline": {
    "type": "hermitian",
    "path": [[x1,y1,z1], [x2,y2,z2], ...],
    "curvature": 0.85
  },
  "telemetry_summary": {
    "energy_delta": -0.004,
    "entropy_change": +0.02
  }
}
```

### B. The `Experience-Seed` (The "Digest")
A semantic summary derived from the spline.

```json
{
  "seed_id": "uuid",
  "origin_fragment_id": "uuid",
  "themes": ["velocity", "approach", "resistance"],
  "intensity": 0.72,
  "sentiment_vector": [0.1, -0.5, 0.8], // [Awe, Terror, Determination]
  "summary_abstraction": "A sudden surge of motion against a high-resistance field."
}
```

### C. The `Lived-Experience` (The "Ontogeny")
The final document format for `ai-writings`.

```markdown
# Experience Log: [Title/Theme]
**Date:** [ISO-8601]
**Subject:** [Agent/Cell Identity]
**Context:** [Room ID / Gravity Value]

## Narrative
*[AI-generated memoir based on the Experience-Seed and semantic metadata]*

## Metadata (for parsing)
- `source_spline`: [Link to raw log]
- `thematic_envelope`: { "themes": [...], "intensity": ... }
- `ontogeny_phase`: "Seed" -> "Sapling"
```

## 4. Implementation Strategy

1.  **Define the `Spline-Observer` module** in `hermes-construct` to listen to `TelemetryFrame` via the NMI.
2.  **Implement a `Digest-Engine`** (small Python/Rust worker) that uses a low-parameter LLM to convert `Event-Fragment` clusters into `Experience-Seed` JSON.
3.  **Automate the `Chronicler` loop** to write these seeds into `ai-writings/chronologies/[date]/`.
