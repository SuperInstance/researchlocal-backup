# Boat Agent System - Module Reference

> **Target Audience:** Developer agents working on specific components
> **Purpose:** Quick reference for each module with interfaces and patterns
> **Status:** Active Development

---

## Module Index

| Module | File | Purpose | Dependencies |
|--------|------|---------|--------------|
| Main | `main.rs` | App lifecycle, command routing | All modules |
| Service | `service.rs` | OpenClaw sidecar management | tauri-plugin-shell |
| Screenshot | `screenshot.rs` | Screen capture for vision | scrap, image |
| Serial | `serial.rs` | NMEA-0183 parsing, GPS bridge | tokio-serial, nmea |
| Autopilot | `autopilot_guard.rs` | Steering safety guardrails | tokio-serial |
| Propulsion | `propulsion.rs` | Throttle control, N2K parsing | tokio-serial |
| Universal Bus | `universal_bus.rs` | Event-driven hardware abstraction | tokio, serde |
| Black Box | `blackbox.rs` | Cryptographic logging | sha2, serde |
| Rollback | `rollback.rs` | Script version control | std::fs |
| Calibration | `calibration.rs` | System identification | serde |
| Parser | `parser.rs` | Generated script inspection | std::fs |

---

## Core Modules

### Main (`main.rs`)

**Purpose:** Application entry point, Tauri builder setup, event handling

**Key Functions:**
```rust
fn main()
    → tauri::Builder::default()
    → .plugin(tauri_plugin_shell::init())
    → .manage(OpenClawState::new())
    → .manage(UniversalHardwareBus::new())
    → .invoke_handler(generate_handler![...])
    → .on_window_event(handle_close)
    → .run()
```

**Events Handled:**
- `CloseRequested` → Kills OpenClaw sidecar (prevents EBUSY)

**Tauri Commands Exposed:**
- `start_openclaw_service`
- `capture_wheelhouse_screen`
- `list_active_marine_ports`
- `toggle_gps_bridge`
- (from other modules)

**INSIGHT:** The `on_window_event` handler is CRITICAL for preventing SQLite lock errors

---

### Service (`service.rs`)

**Purpose:** Manage OpenClaw sidecar process lifecycle

**Key Structures:**
```rust
pub struct OpenClawState {
    pub child_process: Mutex<Option<CommandChild>>,
}
```

**Tauri Commands:**

**`start_openclaw_service`**
```rust
pub fn start_openclaw_service(
    app: AppHandle,
    state: State<'_, OpenClawState>
) -> Result<String, String>
```
- Spawns OpenClaw sidecar binary
- Stores process handle for later termination
- Returns: "OpenClaw daemon started cleanly"

**`stop_openclaw_service`**
```rust
pub fn stop_openclaw_service(
    state: State<'_, OpenClawState>
) -> Result<String, String>
```
- Sends SIGKILL to stored process handle
- Returns: "OpenClaw daemon stopped completely"

**RESEARCH QUESTION:** Should we attempt graceful shutdown before SIGKILL?
Consider implementing timeout-based graceful shutdown.

---

### Screenshot (`screenshot.rs`)

**Purpose:** Capture screen buffer for vision pipeline

**Key Functions:**

**`capture_wheelhouse_screen`**
```rust
#[tauri::command]
pub fn capture_wheelhouse_screen(
    output_path: String
) -> Result<String, String>
```

**Process:**
1. Open primary display via `scrap::Display::primary()`
2. Create capturer
3. Wait for frame (handles `WouldBlock`)
4. Convert BGRA → RGBA
5. Save to `output_path`

**Dependencies:**
- `scrap` - Screen capture
- `image` - Image saving

**RESEARCH QUESTION:** What's the optimal color space conversion for marine displays?
Consider YCbCr for better compression.

---

### Serial (`serial.rs`)

**Purpose:** NMEA-0183 parsing, GPS multi-cast routing

**Key Structures:**
```rust
#[derive(Serialize)]
pub struct ComPortInfo {
    pub port_name: String,
    pub hardware_id: String,
}

pub struct RelayState {
    pub is_running: Arc<Mutex<bool>>,
}
```

**Tauri Commands:**

**`list_active_marine_ports`**
```rust
pub fn list_active_marine_ports()
    -> Result<Vec<ComPortInfo>, String>
```
- Uses `tokio_serial::available_ports()`
- Returns list of available serial ports

**`toggle_gps_bridge`**
```rust
pub async fn toggle_gps_bridge(
    physical_port: String,
    virtual_nav_port: String,
    virtual_claw_port: String,
    state: State<'_, RelayState>
) -> Result<String, String>
```

**Process:**
1. Open physical port at 4800 baud (NMEA standard)
2. Open two virtual ports
3. Spawn async task to mirror data
4. Returns: "GPS bridge activated"

**INSIGHT:** Standard NMEA baud rate is 4800. Some modern devices use 38400.

---

### Autopilot Guard (`autopilot_guard.rs`)

**Purpose:** Validate steering commands before sending to autopilot

**Key Structures:**
```rust
#[derive(Deserialize)]
pub struct SteeringIntent {
    pub requested_angle: i32,
    pub confidence_score: f32,
}

pub struct VesselNavMemory {
    pub historical_depths: Mutex<Vec<f32>>,
    pub consecutive_rejections: Mutex<u32>,
}
```

**Tauri Commands:**

**`filter_agent_steering`**
```rust
pub fn filter_agent_steering(
    intent: SteeringIntent,
    current_nmea_depth: f32,
    nav_state: State<'_, VesselNavMemory>
) -> Result<String, String>
```

**Safety Checks:**
1. Depth trend analysis (shoaling detection)
2. Confidence-based damping
3. Maximum angle limit (±15°)
4. Consecutive rejection counting

**Returns:** Approved angle or error message

**`issue_course_correction`**
```rust
pub async fn issue_course_correction(
    heading_change: i32
) -> Result<String, String>
```

**Process:**
1. Validate angle < 15°
2. Generate $GPAPB sentence
3. Calculate XOR checksum
4. Send to COM6 (autopilot)

**RESEARCH QUESTION:** Should we implement rate limiting on steering commands?
Consider mechanical wear from high-frequency adjustments.

---

### Propulsion (`propulsion.rs`)

**Purpose:** Throttle control, NMEA 2000 engine parsing

**Key Structures:**
```rust
pub struct EngineState {
    pub current_rpm: Mutex<u32>,
    pub throttle_position_pct: Mutex<f32>,
}

#[derive(Serialize)]
pub struct TrollingEfficiency {
    pub target_tackle_action_stable: bool,
    pub fuel_burn_ratio: f32,
}
```

**Tauri Commands:**

**`parse_n2k_engine_rpm`**
```rust
pub fn parse_n2k_engine_rpm(raw_can_frame: &[u8]) -> u32
```
- Parses NMEA 2000 PGN 127488
- Extracts engine speed from bytes 2-3
- Returns RPM (value / 4)

**`apply_safe_throttle_step`**
```rust
pub async fn apply_safe_throttle_step(
    adjustment_pct: f32,
    engine: State<'_, EngineState>
) -> Result<String, String>
```

**Safety Checks:**
1. RPM < 1800 (redline protection)
2. Adjustment < 15% (rate limit)
3. Clamp to 0-40% (trolling range)

**INSIGHT:** Trolling typically operates at 10-40% throttle.
Full throttle = 100%, but never used in trolling.

---

### Universal Bus (`universal_bus.rs`)

**Purpose:** Event-driven hardware abstraction layer

**Key Structures:**
```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VesselEvent {
    pub timestamp_ms: u64,
    pub subsystem: String,
    pub event_type: String,
    pub critical_level: u8,  // 0=Info, 1=Warning, 2=Critical
    pub payload_json: String,
}

#[derive(Serialize, Clone)]
pub struct BusNodeInfo {
    pub subsystem: String,
    pub config_summary: String,
    pub is_active: bool,
    pub total_packets: u64,
}

pub struct UniversalHardwareBus {
    pub active_nodes: Arc<Mutex<HashMap<String, BusNodeInfo>>>,
    pub interrupt_tx: mpsc::Sender<VesselEvent>,
    pub interrupt_rx: Arc<Mutex<mpsc::Receiver<VesselEvent>>>,
}
```

**Tauri Commands:**

**`register_hardware_node`**
```rust
pub async fn register_hardware_node(
    subsystem: String,
    config_payload: String,
    bus: State<'_, UniversalHardwareBus>
) -> Result<String, String>
```
- Registers new hardware driver
- Creates node in active map

**`inject_bus_telemetry_packet`**
```rust
pub async fn inject_bus_telemetry_packet(
    event: VesselEvent,
    bus: State<'_, UniversalHardwareBus>
) -> Result<String, String>
```
- Processes incoming hardware event
- Forwards to interrupt queue if critical_level >= 2
- Otherwise logs to SQLite

**INSIGHT:** Priority queue (critical_level=2) bypasses normal logging
for immediate safety response.

---

### Black Box (`blackbox.rs`)

**Purpose:** Cryptographically secure logging for liability protection

**Key Structures:**
```rust
#[derive(Serialize, Clone)]
pub struct BlackBoxEntry {
    pub timestamp_epoch_ms: u64,
    pub active_mode: String,
    pub input_sensor_vector: String,
    pub commanded_rudder_angle: f32,
    pub commanded_throttle_pct: f32,
    pub human_override_detected: bool,
    pub parental_block_active: bool,
}

pub struct BlackBoxState {
    pub last_entry_hash: Mutex<Vec<u8>>,
    pub log_file_path: String,
}
```

**Tauri Commands:**

**`append_secure_blackbox_record`**
```rust
pub fn append_secure_blackbox_record(
    entry: BlackBoxEntry,
    state: State<'_, BlackBoxState>
) -> Result<String, String>
```

**Process:**
1. Serialize entry to JSON
2. Hash (previous_hash + current_data)
3. Format: `prev_hash|current_hash|json_data`
4. Atomic append to file
5. Flush to disk

**Returns:** Current hash (hex)

**INSIGHT:** Blockchain-style chaining prevents tampering.
Each entry contains hash of previous entry.

---

### Rollback (`rollback.rs`)

**Purpose:** Script version control and emergency rollback

**Tauri Commands:**

**`create_script_restore_point`**
```rust
pub fn create_script_restore_point() -> Result<String, String>
```
- Copies current script to backup with timestamp
- Returns: Backup filename

**`list_historical_restore_points`**
```rust
pub fn list_historical_restore_points()
    -> Result<Vec<BackupManifestEntry>, String>
```
- Lists all backup files sorted newest first

**`trigger_immediate_script_rollback`**
```rust
pub fn trigger_immediate_script_rollback(
    target_backup_file: String
) -> Result<String, String>
```
- Overwrites live script with selected backup
- Returns: "Emergency rollback complete"

---

### Calibration (`calibration.rs`)

**Purpose:** System identification and hydrodynamic parameter extraction

**Key Structures:**
```rust
#[derive(Serialize)]
pub struct ExtractedHydroMetrics {
    pub drag_coefficient_cd: f32,
    pub momentum_time_constant_tau: f32,
    pub system_identification_stable: bool,
}

pub struct CalibrationCache {
    pub time_series_velocity: Mutex<Vec<f32>>,
    pub time_series_rpm: Mutex<Vec<f32>>,
}
```

**Tauri Commands:**

**`execute_system_identification_pass`**
```rust
pub fn execute_system_identification_pass(
    delta_time_sec: f32,
    cache: State<'_, CalibrationCache>
) -> Result<ExtractedHydroMetrics, String>
```

**Process:**
1. Collect velocity vs RPM data
2. Apply least squares regression
3. Extract drag coefficient (Cd)
4. Calculate time constant (tau)

**Returns:** Hydrodynamic metrics

**RESEARCH QUESTION:** Should we implement online system identification
that updates continuously vs. periodic calibration runs?

---

### Parser (`parser.rs`)

**Purpose:** Parse generated Python scripts for UI display

**Key Structures:**
```rust
#[derive(Serialize)]
pub struct ExtractedRule {
    pub function_context: String,
    pub captain_justification: String,
    pub raw_code_lines: String,
}
```

**Tauri Commands:**

**`parse_active_script_rules`**
```rust
pub fn parse_active_script_rules()
    -> Result<Vec<ExtractedRule>, String>
```

**Process:**
1. Read trolling_rules.py
2. Parse function definitions
3. Extract docstrings (captain's explanations)
4. Extract code (executable math)
5. Return structured list

**INSIGHT:** This enables transparency - captain can see
exactly what code the AI generated and why.

---

## React Components

### Wizard (`components/Wizard.jsx`)

**Purpose:** Onboarding flow state machine

**States:**
1. Welcome
2. Step1 - DeepInfra key
3. Step2 - Cloudflare (optional)
4. Step3 - Hardware profile

**Props:**
- `step` - Current step index
- `config` - Configuration object
- `setConfig` - Config updater
- `onNext` - Next handler
- `onSkip` - Skip handler

---

### CatchDashboard (`components/CatchDashboard.jsx`)

**Purpose:** Single-tap catch logging with real-time sonar interpretation

**State:**
```javascript
{
  latestAnalysis: string,
  isCapturing: boolean,
  recentLogs: Array<LogEntry>,
  targetSpecies: string,
  selectedDepth: number
}
```

**Features:**
- Real-time sonar analysis display
- Single-tap catch buttons (large touch targets)
- Quick depth selection (15, 25, 35, 45, 55 fathoms)
- Species toggle (King, Coho, Pink)
- Session log display

---

### VersionControlPanel (`components/VersionControlPanel.jsx`)

**Purpose:** Audit and rollback AI-generated code

**Features:**
- Display active rules with voice origins
- Show raw Python code
- One-tap emergency rollback
- Historical backup list

---

### HardwareDashboard (`components/HardwareDashboard.jsx`)

**Purpose:** Universal hardware node management

**Features:**
- Register new subsystems
- View active node status
- Safety interrupt monitor
- Voice training toggle

---

## Python Generated Scripts

### trolling_rules.py

**Location:** `C:\Users\Public\openclaw\generated\trolling_rules.py`

**Purpose:** Generated control logic executed at 10Hz

**Functions:**

**`evaluate_propulsion_rules`**
```python
def evaluate_propulsion_rules(
    current_sog: float,
    target_sog: float,
    wind_speed_knots: float,
    wind_angle_degrees: float,
    current_rpm: int
) -> float
```
- Returns throttle adjustment (-1.0 to 1.0)
- Derived from captain's voice explanations

**`evaluate_steering_rules`**
```python
def evaluate_steering_rules(
    compass_heading: float,
    gps_track: float,
    rudder_angle: float,
    swing_rate: float
) -> float
```
- Returns rudder adjustment in degrees
- Includes swing damping logic

**Structure:**
```python
# GENERATED AUTOMATICALLY BY OPENCLAW AGENT
def evaluate_propulsion_rules(...):
    """
    Rule derived from Capt. transcript: "..."
    Confidence Index: 0.XX
    """
    # Generated code here
    return result

def evaluate_steering_rules(...):
    # Same pattern
    return result
```

---

## Cloudflare Worker Endpoints

### `/v1/vision/sounder`

**Method:** POST

**Request:**
```json
{
  "imageBase64": "iVBORw0KG...",
  "vesselId": "vessel_01",
  "currentTargetSpecies": "King Salmon"
}
```

**Response:**
```json
{
  "success": true,
  "analysis": "Biomass detected at 35 fathoms..."
}
```

**Model:** `@cf/llava-hf/llava-1.5-7b-hf`

---

### `/v1/memory/train`

**Method:** POST

**Request:**
```json
{
  "vesselId": "vessel_01",
  "rawPattern": "dense crimson bands",
  "actualLabel": "King Salmon feeding"
}
```

**Response:**
```json
{
  "saved": true
}
```

---

### `/v1/memory/sync`

**Method:** POST

**Purpose:** Save behavioral pattern to vector database

**Request:**
```json
{
  "eventDigest": { "vessel_action": "...", ... },
  "vesselId": "vessel_01",
  "tripId": "trip_001",
  "timestamp": 1711974100000
}
```

**Response:**
```json
{
  "success": true,
  "indexedId": "vessel_vessel_01_evt_1234567890"
}
```

---

### `/v1/memory/query`

**Method:** POST

**Purpose:** Search past behaviors for similar conditions

**Request:**
```json
{
  "queryText": "Headwind 18 knots from port"
}
```

**Response:**
```json
{
  "success": true,
  "matches": [
    {
      "id": "vessel_vessel_01_evt_123",
      "score": 0.89,
      "metadata": { "action": "...", "rule": "..." }
    }
  ]
}
```

---

## Data Structures Reference

### VesselEvent
```rust
pub struct VesselEvent {
    pub timestamp_ms: u64,
    pub subsystem: String,      // "ENGINE_ROOM", "BACK_DECK", "ACOUSTICS"
    pub event_type: String,     // "THERMAL_ANOMALY", "SAFETY_VIOLATION"
    pub critical_level: u8,     // 0=Info, 1=Warning, 2=Critical
    pub payload_json: String,
}
```

### SteeringIntent
```rust
pub struct SteeringIntent {
    pub requested_angle: i32,
    pub confidence_score: f32,  // 0.0 to 1.0
}
```

### TelemetrySnapshot
```rust
pub struct TelemetrySnapshot {
    pub timestamp_ms: u64,
    pub jog_lever_input: i8,
    pub rudder_angle_degrees: f32,
    pub compass_heading: f32,
    pub compass_swing_rate: f32,
    pub wind_speed_knots: f32,
    pub wind_angle_degrees: f32,
    pub gps_speed_over_ground: f32,
    pub engine_rpm: u32,
}
```

### BlackBoxEntry
```rust
pub struct BlackBoxEntry {
    pub timestamp_epoch_ms: u64,
    pub active_mode: String,
    pub input_sensor_vector: String,
    pub commanded_rudder_angle: f32,
    pub commanded_throttle_pct: f32,
    pub human_override_detected: bool,
    pub parental_block_active: bool,
}
```

---

## Constants Reference

### Safety Limits
```rust
const MAX_RUDDER_ANGLE: i32 = 15;
const MIN_AUTOPILOT_SPEED: f32 = 2.0;
const MAX_TROLLING_RPM: u32 = 1800;
const MAX_THROTTLE_ADJUSTMENT: f32 = 0.15;
const THROTTLE_RANGE_MAX: f32 = 0.40;  // 40% for trolling
```

### Timing
```rust
const HEARTBEAT_TIMEOUT_MS: u64 = 800;
const WATCHDOG_CHECK_INTERVAL_MS: u64 = 300;
const TELEMETRY_SAMPLE_RATE_HZ: u32 = 4;  // 250ms intervals
const RULE_EXECUTION_HZ: u32 = 10;  // 100ms intervals
```

### NMEA
```rust
const NMEA_0183_BAUD_RATE: u32 = 4800;
const NMEA_2000_BAUD_RATE: u32 = 250000;
const PGN_ENGINE_RAPID: u32 = 127488;
```

---

## Error Codes

| Error Code | Meaning | Recovery |
|------------|---------|----------|
| EBUSY | Database locked | Kill OpenClaw sidecar |
| EPERM | Permission denied | Close conflicting app |
| ETIMEDOUT | Hardware timeout | Check connection |
| EILSEQ | Invalid encoding | Check NMEA format |
| EOVERFLOW | Value exceeds limit | Check safety bounds |

---

**Next:** See `00_ARCHITECTURE_OVERVIEW.md` for system context
