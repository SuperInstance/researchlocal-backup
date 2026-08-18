# Boat Agent System - Agent-First Architecture Documentation

> **Target Audience:** AI agents and software engineers working on this codebase
> **Purpose:** Provide structured, annotated documentation for autonomous development and maintenance
> **Status:** Active Development

---

## Executive Summary

This system is a **Vessel Intelligence Operating System** - a modular, event-driven marine automation platform that transforms commercial fishing vessels into intelligent, autonomous systems. The architecture uses **deterministic code generation** rather than real-time AI inference, ensuring predictable, auditable vessel behavior.

### Core Innovation: AI as Compiler, Not Controller

```
┌─────────────────────────────────────────────────────────────────┐
│                    DESIGN PHILOSOPHY                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ❌ NOT THIS:        AI → Real-time Steering Decisions           │
│                                                                   │
│  ✅ THIS:             AI → Generates Python Code → Boat Runs Code │
│                                                                   │
│  Why: 100% predictable execution, zero runtime latency,          │
│       fully auditable rules, complete offline autonomy          │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## System Architecture

### High-Level Topology

```
┌─────────────────────────────────────────────────────────────────────┐
│                    WHEELHOUSE LAPTOP (TAURI CORE)                  │
│                                                                      │
│  ┌─────────────────────┐         ┌───────────────────────────────┐ │
│  │   React Dashboard   │◄────────►│      Rust Core Engine         │ │
│  │   (UI/Wizard)       │  IPC    │  (Lifecycle/Hardware/Safety)  │ │
│  └─────────────────────┘         └───────────────────────────────┘ │
│                                              │                      │
│           ┌─────────────────────────────────┼──────────────┐       │
│           ▼                                 ▼              ▼       │
│  ┌──────────────────┐        ┌──────────────────┐   ┌──────────┐ │
│  │ OpenClaw Sidecar  │        │  Virtual COM     │   │ Hardware │ │
│  │ (AI/Tools)        │        │  Ports          │   │ Drivers  │ │
│  └──────────────────┘        │  COM4/COM5/COM6 │   └──────────┘ │
│                               └──────────────────┘                │
└─────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼ (HTTP POST <15KB)
┌─────────────────────────────────────────────────────────────────────┐
│                    CLOUDFLARE EDGE NETWORK                          │
│                                                                      │
│  ┌───────────────────┐         ┌───────────────────────────────┐  │
│  │ Workers AI        │         │  KV Storage / Vectorize        │  │
│  │ (Vision Inference)│         │  (Vocabulary/Memory)           │  │
│  └───────────────────┘         └───────────────────────────────┘  │
│                                                                      │
│  Free tier: 100k requests/day, free daily AI resets                 │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Registry

### Phase 1: Native Shell & Process Supervisor (Rust/Tauri)

**Purpose:** Zero-configuration desktop application wrapper that prevents database corruption and eliminates terminal interfaces.

**Key Modules:**
- `src-tauri/src/main.rs` - Application lifecycle & event routing
- `src-tauri/src/service.rs` - OpenClaw sidecar process management
- `src-tauri/src/screenshot.rs` - Win32 GDI screen capture
- `src-tauri/src/serial.rs` - Asynchronous NMEA-0183 serial parsing

**Critical Function:** Clean SIGKILL on window close to prevent SQLite EBUSY lock errors

```rust
// RESEARCH QUESTION: Should we implement graceful shutdown sequence
// for sidecar processes before SIGKILL? Consider data loss scenarios.
```

---

### Phase 2: Adaptive Onboarding State Machine (React)

**Purpose:** Captain-proof installation wizard for non-technical users.

**States:**
1. Welcome Screen
2. Engine Selection (DeepInfra API Key)
3. Cloud Sync (Cloudflare - Optional)
4. Hardware Profiling (GPU/RAM detection)
5. Initialization

**Implementation:** `src/components/Wizard.jsx` with sub-components for each step

**INSIGHT:** Hardware-based adaptive routing (GPU+16GB RAM → Ollama local, else cloud-only)

---

### Phase 3: High-Compression Vision Pipeline

**Purpose:** Convert depth sounder screens to text data over satellite links.

**Flow:**
```
Screen Capture (Rust) → Crop & Quantize → <15KB Base64
    → Cloudflare Worker (@cf/llava-hf/llava-1.5-7b-hf)
    → Structured Text → Boat
```

**RESEARCH QUESTION:** Can we achieve lower compression using PNG compression
vs JPEG for sonar data? Quantify trade-offs.

---

### Phase 4: Serial COM Multi-Cast Splicer

**Purpose:** Split single GPS signal to Nav software AND OpenClaw.

**Architecture:**
```
Physical COM3 → Virtual COM4 (Nav) + Virtual COM5 (OpenClaw)
```

**Implementation:** Tokio-based async router with com0com virtual port creation

---

### Phase 5: Autopilot Integration

**Purpose:** Closed-loop steering control with safety guardrails.

**Safety Layers:**
1. OpenClaw generates steering intent (JSON)
2. Rust validates against hard limits (±15°, speed >2 knots, 60s cooldown)
3. NMEA sentence generation ($GPAPB with XOR checksum)
4. Serial output to autopilot

**CRITICAL SAFETY:** Human jog lever = Absolute hardware interrupt

---

### Phase 6: Propulsion Control

**Purpose:** Dynamic throttle adjustment for trolling speed optimization.

**Components:**
- NMEA 2000 PGN 127488 parser (Engine Parameters Rapid Update)
- PID controller for hydrodynamic lag compensation
- Linear servo actuator for mechanical throttles
- Digital potentiometer for fly-by-wire systems

**RESEARCH QUESTION:** Implement formal verification of PID stability bounds
for vessel-specific hydrodynamics.

---

### Phase 7: Behavioral Cloning & Imitation Learning

**Purpose:** Learn captain's fishing style from voice + telemetry.

**Data Structure:**
```json
{
  "timestamp_ms": 1711974008000,
  "telemetry": { /* full vessel state */ },
  "captain_voice_transcript": "Ramping throttle before head-wind turn..."
}
```

**Key Innovation:** Multi-modal context alignment with word-level timestamp synchronization

**INSIGHT:** Voice annotations provide LABELS for telemetry, enabling
supervised learning of vessel-specific behaviors.

---

### Phase 8: Deterministic Code Generation

**Purpose:** AI writes Python control scripts instead of controlling hardware directly.

**Generated Script:** `C:\Users\Public\openclaw\generated\trolling_rules.py`

**Functions:**
- `evaluate_propulsion_rules(current_sog, target_sog, wind_speed_knots, ...)`
- `evaluate_steering_rules(compass_heading, gps_track, rudder_angle, ...)`

**Execution:** Embedded Python (pyo3) running at 10Hz in Rust core

**CRITICAL INSIGHT:** This eliminates AI hallucination risk at runtime.
The AI becomes a software engineer, not a controller.

---

### Phase 9: Universal Hardware Bus

**Purpose:** Modular, event-driven architecture for ANY vessel sensor.

**Event Contract:**
```rust
pub struct VesselEvent {
    pub timestamp_ms: u64,
    pub subsystem: String,      // "ENGINE_ROOM", "BACK_DECK", "ACOUSTICS"
    pub event_type: String,     // "THERMAL_ANOMALY", "SAFETY_VIOLATION"
    pub critical_level: u8,     // 0 = Nominal, 1 = Warning, 2 = Critical
    pub payload_json: String,
}
```

**Supported Inputs:**
- IP Cameras (RTSP) - Engine room thermal
- NMEA 2000 Bus - Engine/hydraulics/tanks
- Drone Telemetry (MavLink) - Aerial surveillance
- Custom sensors via microcontrollers

---

### Phase 10: Safety & Liability Layer

**Purpose:** Cryptographic black box for legal protection and hardware safety.

**Black Box Entry:**
```rust
pub struct BlackBoxEntry {
    pub timestamp_epoch_ms: u64,
    pub active_mode: String,        // "MANUAL", "AI_ACTIVE", "COACHING"
    pub input_sensor_vector: String,
    pub commanded_rudder_angle: f32,
    pub commanded_throttle_pct: f32,
    pub human_override_detected: bool,
}
```

**Implementation:** SHA-256 chained log (blockchain-style) for tamper-proof records

---

## Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| UI | React + Tailwind | Touchscreen dashboard |
| Core | Tauri v2 (Rust) | Native system control |
| AI Runtime | OpenClaw | Agent execution |
| Vision | Cloudflare Workers AI | Free-tier multimodal |
| Local Fallback | Ollama (llama3.2:3b) | Offline reasoning |
| Hardware | tokio-serial | Async serial I/O |
| Scripts | Python (pyo3) | Generated control logic |
| Storage | SQLite + Cloudflare KV | Local + cloud persistence |

---

## Data Flow Diagrams

### Vision Processing Flow
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Sounder     │     │ Tauri Rust  │     │ Cloudflare  │
│ Screen      │────►│ Screen Cap  │────►│ Workers AI  │
│ (Physical)  │     │ + Compress  │     │ (LLaVA)     │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
                                       "Biomass at 35f"
                                              │
                                              ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Autopilot   │◄────│ Policy      │◄────│ OpenClaw    │
│ Adjust      │     │ Engine      │     │ Agent       │
└─────────────┘     └─────────────┘     └─────────────┘
```

### Learning Loop Flow
```
┌──────────────┐
│ Captain      │
│ Drives +     │
│ Speaks       │
└──────┬───────┘
       │ Telemetry + Voice
       ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Rolling      │────►│ OpenClaw     │────►│ Generated    │
│ Buffer       │     │ (Offline)    │     │ Python Code  │
└──────────────┘     └──────────────┘     └──────────────┘
                                              │
                                              ▼
                                       ┌──────────────┐
                                       │ Embedded     │
                                       │ Python (10Hz)│
                                       └──────────────┘
```

---

## Safety Architecture

### Hardware Override Priority
```
1. HUMAN JOG LEVER (Absolute interrupt)
2. HARD WATCHDOG TIMEOUT (800ms)
3. SENSOR SANITY CHECKS
4. AI-GENERATED RULES
```

### Failure Modes
| Failure Mode | Detection | Response |
|--------------|-----------|----------|
| Laptop freeze | Watchdog timeout | Relay opens, buzzer sounds |
| Sensor corruption | Kalman filter | Safe degraded mode |
| Network loss | Ping check | Local Ollama fallback |
| Invalid AI output | Rust bounds checking | Command rejection |

---

## Deployment Architecture

### Build Process
```bash
npm run fleet:compile
# 1. Frontend optimization (Vite)
# 2. Rust compilation (all safety modules)
# 3. Sidecar bundling (OpenClaw, Ollama, com0com)
# 4. MSI packaging (Windows installer)
```

### Distribution
- Single `.msi` file
- Silent driver installation
- Zero terminal visibility
- One-click captain setup

---

## Key Research Questions & Insights

### 1. Time-Synchronization Precision
**Question:** Word-level audio timestamps from Whisper API - what's the actual precision?
**Impact:** Misaligned voice-telemetry pairs = confused AI
**Investigation Needed:** Measure alignment error vs training quality

### 2. Sonar Vision Training Data
**Question:** Generic models fail on marine displays. Need fine-tuning dataset?
**Impact:** False positives = wasted fuel, missed fish
**Investigation Needed:** Collect labeled echogram dataset

### 3. PID Stability Across Conditions
**Question:** Single PID parameters for all sea states?
**Impact:** Oscillation in rough seas, sluggish in calm
**Investigation Needed:** Adaptive PID or gain scheduling?

### 4. Kalman Filter for Sensors
**Question:** Best filter topology for marine sensor fusion?
**Impact:** Sensor failures = unsafe behavior
**Investigation Needed:** Compare EKF vs UKF for vessel state estimation

### 5. Code Generation Safety
**Question:** How to prove generated code is safe?
**Impact:** Bad code = vessel damage
**Investigation Needed:** Formal verification, runtime assertion injection

---

## Implementation Priorities

### P0 - Safety Critical (Must Have)
1. Hardware watchdog timeout (800ms)
2. Jog lever absolute interrupt
3. Cryptographic black box logging
4. Sensor sanity validation
5. Rust bounds checking

### P1 - Core Functionality (High Priority)
1. Basic autopilot steering
2. Vision-to-text pipeline
3. Single-catch logging UI
4. GPS bridge functionality
5. Local rule execution

### P2 - Advanced Features (Medium Priority)
1. Behavioral cloning
2. Voice annotation
3. Cloud sync
4. Multi-vessel memory
5. Advanced safety filters

### P3 - Optimization (Low Priority)
1. Compression improvements
2. Model fine-tuning
3. Custom hardware
4. Fleet analytics
5. Mobile app

---

## File Structure Reference

```
boat-agent-app/
├── package.json
├── src/
│   ├── main.js
│   └── components/
│       ├── Wizard.jsx
│       ├── CatchDashboard.jsx
│       ├── VersionControlPanel.jsx
│       └── SeaTrialsPanel.jsx
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs
│   │   ├── service.rs
│   │   ├── screenshot.rs
│   │   ├── serial.rs
│   │   ├── autopilot_guard.rs
│   │   ├── propulsion.rs
│   │   ├── universal_bus.rs
│   │   ├── blackbox.rs
│   │   ├── rollback.rs
│   │   └── calibration.rs
│   └── binaries/
│       ├── openclaw-x86_64-pc-windows-msvc.exe
│       ├── ollama-x86_64-pc-windows-msvc.exe
│       └── setupc-x86_64-pc-windows-msvc.exe
└── openclaw/
    ├── prompts/
    │   └── codegen_rules.txt
    ├── generated/
    │   └── trolling_rules.py
    └── backups/
        └── trolling_rules_backup_*.py
```

---

## Glossary

| Term | Definition |
|------|------------|
| **VesselEvent** | Standardized JSON contract for all hardware inputs |
| **BlackBoxEntry** | Cryptographically signed log entry for liability |
| **Code-Generation Loop** | AI writes Python scripts instead of controlling hardware |
| **Hydrodynamic Lag** | Delay between throttle change and vessel response |
| **Swing Rate** | Speed of compass needle oscillation (deg/sec) |
| **Ziegler-Nichols** | PID tuning methodology for oscillation-based calibration |
| **N2K / NMEA 2000** | CAN bus protocol for marine electronics |
| **PGN 127488** | Engine Parameters Rapid Update message |
| **RAI** | Rudder Angle Indicator sensor |
| **LLaVA** | Large Language and Vision Assistant for multimodal AI |

---

**Next:** See `01_IMPLEMENTATION_GUIDE.md` for concrete implementation steps
**Next:** See `02_RESEARCH_QUESTIONS.md` for open questions and investigation areas
