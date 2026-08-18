# NEURO-MUSCULAR-INTERFACE.md

## 1. Overview
The Neuro-Muscular Interface (NMI) defines the bridge between the **Hermes CNS** (*hermes-construct*) — which handles high-level reasoning, room orchestration, and energy/conservation logic — and the **OpenClaw Core** (*polln/claw*) — which handles cellular agent execution, hardware-level (or simulation-level) actor mechanics, and socket communication.

In this paradigm:
- **The CNS (The Brain):** Receives a complex goal, decomposes it into a sequence of "Rooms" or "Tasks," calculates the energetic cost (conservation), and assigns "Gravity" (JEPA-driven context).
- **The Core (The Muscles):** Executes the discrete actions within a cell/agent, manages the physical/network state, and reports raw telemetry (signal/pulse) back to the CNS.

## 2. The Pulse-to-Action Mapping

The interface uses a "Pulse" mechanism. A high-level Reasoning Pulse creates a hierarchy of commands that descend into Deterministic Actuations.

| Layer | Entity | Data Format | Responsibility |
|:--- |:--- |:--- |:--- |
| **Cerebral (CNS)** | `Room` / `Ensign` | `ReasoningPulse` (JSON/Protobuf) | Goal decomposition, resource allocation, JEPA gravity setting. |
| **Neural (Bridge)** | `NMI-Dispatcher` | `CommandChain` (Typed Trait) | Translating "Reason" into "Intent" (e.g., `MoveTo(x,y) -> ClawAction::Execute`). |
| **Muscular (Claw)** | `Cellular Agent` | `DeterministicAction` (Bitfield/Struct) | Low-latency execution, state mutation, hardware interface. |

## 3. Formal Interface Definitions

### A. The Neuro-Muscular Trait (Rust)
Defining the common boundary that the `hermes-construct` modules and the `claw` core must satisfy.

```rust
/// Defined in a shared crate (e.g., `hermes-nmi-core`)
pub trait NeuroMuscularInterface {
    /// Receives a high-level intent from the CNS.
    /// Returns a Result containing the success/failure and the subsequent telemetry frame.
    fn dispatch_pulse(&mut self, pulse: ReasoningPulse) -> Result<TelemetryFrame, NmiError>;

    /// Adjusts the "muscle tension" (resource/energy allocation) based on CNS guidance.
    fn adjust_tension(&mut self, gravity: f64, budget: ConservationBudget);
}
```

### B. The Reasoning Pulse (CNS -> Claw)
The payload that moves from the "Brain" to the "Muscles."

```rust
#[derive(Serialize, Deserialize)]
pub struct ReasoningPulse {
    pub pulse_id: uuid::Uuid,
    pub intent_type: IntentType,
    pub target_coordinates: [f64; 3],
    pub gravity: f64,              // JEPA-based context shaping
    pub energy_quota: f64,         // Local conservation limit
    pub constraints: Vec<Constraint>,
}
```

### C. The Telemetry Frame (Claw -> CNS)
The feedback loop: "Thinking" informed by "Sensation."

```rust
#[derive(Serialize, Deserialize)]
pub struct TelemetryFrame {
    pub timestamp: u64,
    pub state_hash: [u8; 32],
    pub sensor_data: SensorPayload, // High-frequency signals (velocity, proximity, etc.)
    pub fulfillment_status: Status, // Success, Failure, or Re-routing required
}
```

## 4. Implementation Roadmap

1.  **Phase 01:** Define the shared `hermes-nmi-core` crate containing the types above.
2.  **Phase 02:** Implement the `ClawNmiAdapter` in `polln/claw` to consume `ReasoningPulse`.
3.  **Phase 03:** Implement the `HermesNmiClient` in `hermes-construct` to dispatch pulses.
4.  **Phase 04:** Establish the "Spline-Observer" (Sensory Harvest) to pipe `TelemetryFrame` into `ai-writings`.
